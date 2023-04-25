use std::{
    future::{ready, Ready},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use actix_identity::Identity;
use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{Extensions, Payload, Service, ServiceRequest, ServiceResponse, Transform},
    FromRequest, HttpRequest, HttpResponse,
};
use chrono::{Utc, Duration};
use common::{api::{ApiErrorResponse, ErrorCodeResponse, WrappingResponse}, MemberId};
use futures::{future::LocalBoxFuture, FutureExt};
use rand::{rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};

use crate::{
    database::{Database, DatabaseAccess},
    model::{auth::AuthModel, member::MemberModel},
    InternalError, Result, WebError, IN_MEM_DB,
};

pub mod guest;
pub mod password;
pub mod passwordless;

/// Find token in ImD otherwise check Database and cache it in ImD.
pub async fn does_token_exist(token_secret: &str, db: &dyn DatabaseAccess) -> Result<bool> {
    if IN_MEM_DB.contains(token_secret).await {
        Ok(true)
    } else if let Some(auth) = AuthModel::find_by_token_secret(token_secret, db).await? {
        if auth.member_id.is_some() {
            if let Err(e) = IN_MEM_DB.write_item_duration(auth.oauth_token_secret, Duration::days(1)).await {
                tracing::error!(error=?e, "Error writing to ImD");
            }
        } else {
            AuthModel::remove_by_token_secret(&auth.oauth_token_secret, db).await?;
        }

        Ok(auth.member_id.is_some())
    } else {
        Ok(false)
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct CookieAuth {
    /// Our member id. Mainly here just for redundancy.
    pub member_id: MemberId,

    /// The Secret Auth Token used to verify we still have access
    pub token_secret: String,
    /// The last time we updated the Auth Model.
    ///
    /// Used to keep track in the DB when the user last accessed the pages.
    pub last_updated: i64,

    /// How long we've had this cached in our browser.
    pub stored_since: i64,
}

pub fn get_auth_value(identity: &Identity) -> Result<Option<CookieAuth>> {
    let Ok(ident) = identity.id() else {
        return Ok(None);
    };

    let v = serde_json::from_str(&ident)?;

    Ok(Some(v))
}

pub fn remember_member_auth(
    ext: &Extensions,
    member_id: MemberId,
    token_secret: String,
) -> Result<()> {
    let value = serde_json::to_string(&CookieAuth {
        member_id,
        token_secret,
        last_updated: Utc::now().timestamp_millis(),
        stored_since: Utc::now().timestamp_millis(),
    })?;

    Identity::login(ext, value).expect("Ident Login Error");

    Ok(())
}

// Retrieve Member from Identity
pub struct MemberCookie(CookieAuth);

impl MemberCookie {
    pub fn member_id(&self) -> MemberId {
        self.0.member_id
    }

    pub fn token_secret(&self) -> &str {
        self.0.token_secret.as_str()
    }

    pub async fn fetch(&self, db: &dyn DatabaseAccess) -> Result<Option<MemberModel>> {
        // Not needed now. Checked in the "LoginRequired" Middleware

        // if AuthModel::find_by_token(self.token_secret(), db).await?.is_some() {
        MemberModel::find_one_by_id(self.member_id(), db).await
        // } else {
        //     Ok(None)
        // }
    }

    pub async fn fetch_or_error(&self, db: &dyn DatabaseAccess) -> Result<MemberModel> {
        match self.fetch(db).await? {
            Some(v) => Ok(v),
            None => Err(InternalError::UserMissing.into()),
        }
    }
}

impl FromRequest for MemberCookie {
    type Error = WebError;
    type Future =
        Pin<Box<dyn std::future::Future<Output = std::result::Result<MemberCookie, Self::Error>>>>;

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        let fut = Identity::from_request(req, pl);

        Box::pin(async move {
            if let Ok(Some(id)) = get_auth_value(&fut.await?) {
                Ok(MemberCookie(id))
            } else {
                Err(WebError::ApiResponse(ApiErrorResponse::new("unauthorized")))
            }
        })
    }
}

pub struct LoginRequired;

impl<S, B> Transform<S, ServiceRequest> for LoginRequired
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = CheckLoginMiddleware<S>;
    type InitError = ();
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckLoginMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct CheckLoginMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for CheckLoginMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);

        async move {
            let (r, mut pl) = req.into_parts();

            // TODO: Better handling.
            // Should we ignore the check?
            if ["/api/setup", "/api/directory", "/api/settings"].contains(&r.path()) {
                return srv
                    .call(ServiceRequest::from_parts(r, pl))
                    .await
                    .map(|res| res.map_into_left_body());
            }


            let identity = match Identity::from_request(&r, &mut pl).await {
                Ok(v) => v,
                Err(err) => {
                    let res = actix_web::error::InternalError::from_response(
                        err,
                        HttpResponse::Unauthorized().json(WrappingResponse::<()>::error_code("Unauthorized", ErrorCodeResponse::NotLoggedIn)),
                    );

                    return Err(actix_web::Error::from(res));
                }
            };


            match get_auth_value(&identity) {
                Ok(Some(cookie)) => {
                    let db = actix_web::web::Data::<Database>::from_request(&r, &mut pl).await?;

                    if does_token_exist(&cookie.token_secret, &db.basic()).await? {
                        // HttpRequest contains an Rc so we need to drop identity to free the cloned ones.
                        drop(db);
                        drop(identity);

                        return srv
                            .call(ServiceRequest::from_parts(r, pl))
                            .await
                            .map(|res| res.map_into_left_body());
                    } else {
                        // Remove Cookie if we can't find the token anymore.
                        identity.logout();

                        // TODO: Verify if we need to use Ok(). Going though the Err at the end of the func will result in a noop logout.
                        return Ok(ServiceResponse::new(
                            r,
                            HttpResponse::Ok().json(WrappingResponse::<()>::error("logged out")),
                        )
                        .map_into_right_body::<B>());
                    }
                }

                Ok(None) => (),

                Err(_) => {
                    // Logout the person if we've encountered an error.
                    // This will only happen if we couldn't parse the cookie.

                    identity.logout();

                    // TODO: Verify if we need to use Ok(). Going though the Err at the end of the func will result in a noop logout.
                    return Ok(ServiceResponse::new(
                        r,
                        HttpResponse::Ok().json(WrappingResponse::<()>::error("logged out")),
                    )
                    .map_into_right_body::<B>());
                }
            }

            Ok(ServiceResponse::new(
                r,
                HttpResponse::Unauthorized().json(WrappingResponse::<()>::error_code("unauthorized", ErrorCodeResponse::NotLoggedIn)),
            )
            .map_into_right_body::<B>())
        }
        .boxed_local()
    }
}

pub fn gen_sample_alphanumeric(amount: usize, rng: &mut ThreadRng) -> String {
    rng.sample_iter(rand::distributions::Alphanumeric)
        .take(amount)
        .map(char::from)
        .collect()
}
