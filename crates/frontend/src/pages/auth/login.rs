use common::api::ApiErrorResponse;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::use_async;
use yew_router::{prelude::RouterScopeExt, history::History};

use crate::{request, Route};

pub enum Msg {
    LoginPasswordResponse(std::result::Result<String, ApiErrorResponse>),
    LoginPasswordlessResponse(std::result::Result<String, ApiErrorResponse>),
}

pub struct LoginPage {
    password_response: Option<std::result::Result<String, ApiErrorResponse>>,
    passwordless_response: Option<std::result::Result<String, ApiErrorResponse>>,
    // prevent_submit: bool,
}

impl Component for LoginPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            password_response: None,
            passwordless_response: None,
            // prevent_submit: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoginPasswordResponse(resp) => {
                if resp.is_ok() {
                    crate::request_member_self();

                    let history = ctx.link().history().unwrap();
                    history.push(Route::Dashboard);
                }

                self.password_response = Some(resp);
            }

            Msg::LoginPasswordlessResponse(resp) => {
                if resp.is_ok() {
                    crate::request_member_self();

                    let history = ctx.link().history().unwrap();
                    history.push(Route::Dashboard);
                }

                self.passwordless_response = Some(resp);
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="login-container">
                <div class="center-normal">
                    <div class="center-container">
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
    let passless_email = use_state(String::new);

    let on_change_passless_email = {
        let value = passless_email.setter();
        Callback::from(move |e: Event| value.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };

    let submit_passless = use_async(async move {
        let email = passless_email.clone();
        request::login_without_password(email.to_string()).await.ok()
    });

    let async_resp = submit_passless.clone();
    let callback = props.cb.clone();
    use_effect_with_deps(move |loading| {
        if !*loading && (async_resp.data.is_some() || async_resp.error.is_some()) {
            callback.emit(async_resp.data.clone().ok_or_else(|| async_resp.error.clone().unwrap()));
        }

        || {}
    }, submit_passless.loading);

    html! {
        <>
            <h2>{ "Passwordless Login" }</h2>
            <div class="form-container">
                <label for="emailpassless">{ "Email Address" }</label>
                <input type="email" name="email" id="emailpassless" onchange={ on_change_passless_email } />

                <input type="submit" value="Log in" class="button" onclick={ Callback::from(move |_| submit_passless.run()) } />
            </div>
        </>
    }
}


#[function_component(PasswordLogin)]
pub fn _password(props: &InnerProps) -> Html {
    let pass_email = use_state(String::new);
    let pass_pass = use_state(String::new);

    let on_change_pass_email = {
        let value = pass_email.setter();
        Callback::from(move |e: Event| value.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };

    let on_change_pass_pass = {
        let value = pass_pass.setter();
        Callback::from(move |e: Event| value.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };

    let submit_pass = use_async(async move {
        let email = pass_email.clone();
        let pass = pass_pass.clone();
        request::login_with_password(email.to_string(), pass.to_string()).await.ok()
    });

    let async_resp = submit_pass.clone();
    let callback = props.cb.clone();
    use_effect_with_deps(move |loading| {
        if !*loading && (async_resp.data.is_some() || async_resp.error.is_some()) {
            callback.emit(async_resp.data.clone().ok_or_else(|| async_resp.error.clone().unwrap()));
        }

        || {}
    }, submit_pass.loading);

    html! {
        <>
            <h2>{ "Password Login" }</h2>
            <div class="form-container">
                <label for="email">{ "Email Address" }</label>
                <input type="email" name="email" id="email" onchange={ on_change_pass_email } />

                <label for="password">{ "Password" }</label>
                <input type="password" name="password" id="password" onchange={ on_change_pass_pass } />

                <input type="submit" value="Log in" class="button" onclick={ Callback::from(move |_| submit_pass.run()) } />
            </div>
        </>
    }
}