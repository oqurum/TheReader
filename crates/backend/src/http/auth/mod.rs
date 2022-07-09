use std::{pin::Pin, future::{Ready, ready}, task::{Poll, Context}, rc::Rc};

use actix_identity::Identity;
use actix_web::{FromRequest, HttpRequest, dev::{Payload, Transform, Service, ServiceRequest, ServiceResponse}, error::ErrorUnauthorized, body::MessageBody};
use chrono::Utc;
use common::MemberId;
use futures::{future::LocalBoxFuture, FutureExt};
use serde::{Deserialize, Serialize};

use crate::{Result, database::Database, model::member::MemberModel};

pub mod password;
pub mod passwordless;


#[derive(Serialize, Deserialize)]
pub struct CookieAuth {
	pub member_id: MemberId,
	pub stored_since: i64,
}


pub fn get_auth_value(identity: &Identity) -> Option<CookieAuth> {
	let ident = identity.identity()?;
	serde_json::from_str(&ident).ok()
}

pub async fn get_auth_member(identity: &Identity, db: &Database) -> Option<MemberModel> {
	let store = get_auth_value(identity)?;
	MemberModel::find_by_id(store.member_id, db).await.ok().flatten()
}

pub fn remember_member_auth(member_id: MemberId, identity: &Identity) -> Result<()> {
	let value = serde_json::to_string(&CookieAuth {
		member_id,
		stored_since: Utc::now().timestamp_millis(),
	})?;

	identity.remember(value);

	Ok(())
}

// Retrive Member from Identity
pub struct MemberCookie(CookieAuth);

impl MemberCookie {
	pub fn member_id(&self) -> MemberId {
		self.0.member_id
	}
}


impl FromRequest for MemberCookie {
	type Error = actix_web::Error;
	type Future = Pin<Box<dyn std::future::Future<Output = std::result::Result<MemberCookie, actix_web::Error>>>>;

	fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
		let fut = Identity::from_request(req, pl);

		Box::pin(async move {
			if let Some(id) = get_auth_value(&fut.await?) {
				Ok(MemberCookie(id))
			} else {
				Err(ErrorUnauthorized("unauthorized"))
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
	type Response = ServiceResponse<B>;
	type Error = actix_web::Error;
	type Transform = CheckLoginMiddleware<S>;
	type InitError = ();
	type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		ready(Ok(CheckLoginMiddleware { service: Rc::new(service) }))
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
	type Response = ServiceResponse<B>;
	type Error = actix_web::Error;
	type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

	fn poll_ready(
		&self,
		cx: &mut Context<'_>,
	) -> Poll<std::result::Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&self, req: ServiceRequest) -> Self::Future {
		let srv = Rc::clone(&self.service);

		async move {
			let (r, mut pl) = req.into_parts();

			let identity = Identity::from_request(&r, &mut pl).await?;

			if get_auth_value(&identity).is_some() {
				// HttpRequest contains an Rc so we need to drop identity to free the cloned one.
				drop(identity);

				Ok(srv.call(ServiceRequest::from_parts(r, pl)).await?)
			} else {
				Err(ErrorUnauthorized("unauthorized"))
			}
		}
		.boxed_local()
	}
}