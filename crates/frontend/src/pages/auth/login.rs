use common::api::ApiErrorResponse;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::use_async;
use yew_router::{history::History, prelude::RouterScopeExt};

use crate::{request, BaseRoute};

pub enum Msg {
    LoginPasswordResponse(std::result::Result<String, ApiErrorResponse>),
    LoginPasswordlessResponse(std::result::Result<String, ApiErrorResponse>),
}

pub struct LoginPage;

impl Component for LoginPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoginPasswordResponse(resp) => {
                if resp.is_ok() {
                    crate::request_member_self();

                    let history = ctx.link().history().unwrap();
                    history.push(BaseRoute::Dashboard);
                }
            }

            Msg::LoginPasswordlessResponse(resp) => {
                if resp.is_ok() {
                    crate::request_member_self();

                    let history = ctx.link().history().unwrap();
                    history.push(BaseRoute::Dashboard);
                }
            }
        }

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="login-container">
                <div class="center-normal">
                    <div class="center-container">
                        // TODO: Impl. display for selecting with login to use. Don't display both at the same time.
                        <PasswordlessLogin cb={ ctx.link().callback(Msg::LoginPasswordlessResponse) } />
                        <PasswordLogin cb={ ctx.link().callback(Msg::LoginPasswordResponse) } />
                    </div>
                </div>
            </div>
        }
    }
}

#[derive(Properties)]
pub struct InnerProps {
    pub cb: Callback<std::result::Result<String, ApiErrorResponse>>,
}

impl PartialEq for InnerProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[function_component(PasswordlessLogin)]
pub fn _passwordless(props: &InnerProps) -> Html {
    let response_error = use_state_eq(|| Option::<String>::None);

    let passless_email = use_state(String::new);

    let on_change_passless_email = {
        let value = passless_email.setter();
        Callback::from(move |e: Event| {
            value.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };

    let submit_passless = use_async(async move {
        let email = passless_email.clone();
        request::login_without_password(email.to_string())
            .await
            .ok()
    });

    let async_resp = submit_passless.clone();
    let callback = props.cb.clone();
    let data_resp_error = response_error.setter();
    use_effect_with_deps(
        move |loading| {
            if !*loading && (async_resp.data.is_some() || async_resp.error.is_some()) {
                data_resp_error.set(async_resp.error.as_ref().map(|v| v.description.clone()));

                callback.emit(
                    async_resp
                        .data
                        .clone()
                        .ok_or_else(|| async_resp.error.clone().unwrap()),
                );
            }

            || {}
        },
        submit_passless.loading,
    );

    html! {
        <>
            <h2>{ "Passwordless Login" }</h2>

            <form onsubmit={ Callback::from(move |_| submit_passless.run()) }>
                <div class="mb-3">
                    <label class="form-label" for="emailpassless">{ "Email Address" }</label>
                    <input class="form-control" type="email" name="email" id="emailpassless" onchange={ on_change_passless_email } />
                </div>

                <input type="submit" value="Log in" class="btn btn-primary" />

                {
                    if let Some(error) = response_error.as_ref() {
                        html! {
                            <div class="label red">{ error.clone() }</div>
                        }
                    } else {
                        html! {}
                    }
                }
            </form>
        </>
    }
}

#[function_component(PasswordLogin)]
pub fn _password(props: &InnerProps) -> Html {
    let response_error = use_state_eq(|| Option::<String>::None);

    let pass_email = use_state(String::new);
    let pass_pass = use_state(String::new);

    let on_change_pass_email = {
        let value = pass_email.setter();
        Callback::from(move |e: Event| {
            value.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };

    let on_change_pass_pass = {
        let value = pass_pass.setter();
        Callback::from(move |e: Event| {
            value.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };

    let submit_pass = use_async(async move {
        let email = pass_email.clone();
        let pass = pass_pass.clone();
        request::login_with_password(email.to_string(), pass.to_string())
            .await
            .ok()
    });

    let async_resp = submit_pass.clone();
    let callback = props.cb.clone();
    let data_resp_error = response_error.setter();
    use_effect_with_deps(
        move |loading| {
            if !*loading && (async_resp.data.is_some() || async_resp.error.is_some()) {
                data_resp_error.set(async_resp.error.as_ref().map(|v| v.description.clone()));

                callback.emit(
                    async_resp
                        .data
                        .clone()
                        .ok_or_else(|| async_resp.error.clone().unwrap()),
                );
            }

            || {}
        },
        submit_pass.loading,
    );

    html! {
        <>
            <h2>{ "Password Login" }</h2>
            <form onsubmit={ Callback::from(move |_| submit_pass.run()) }>
                <div class="mb-3">
                    <label class="form-label" for="email">{ "Email Address" }</label>
                    <input class="form-control" type="email" name="email" id="email" onchange={ on_change_pass_email } />
                </div>

                <div class="mb-3">
                    <label class="form-label" for="password">{ "Password" }</label>
                    <input class="form-control" type="password" name="password" id="password" onchange={ on_change_pass_pass } />
                </div>

                <input type="submit" value="Log in" class="btn btn-primary" />

                {
                    if let Some(error) = response_error.as_ref() {
                        html! {
                            <div class="label red">{ error.clone() }</div>
                        }
                    } else {
                        html! {}
                    }
                }
            </form>
        </>
    }
}
