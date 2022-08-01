use common::api::ApiErrorResponse;
use web_sys::HtmlInputElement;
use yew::prelude::*;
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
                    let history = ctx.link().history().unwrap();
                    history.push(Route::Dashboard);
                }

                self.password_response = Some(resp);
            }

            Msg::LoginPasswordlessResponse(resp) => {
                if resp.is_ok() {
                    let history = ctx.link().history().unwrap();
                    history.push(Route::Dashboard);
                }

                self.passwordless_response = Some(resp);
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let passless_email = use_state(String::new);

        let pass_email = use_state(String::new);
        let pass_pass = use_state(String::new);

        let on_change_passless_email = {
            let value = passless_email.setter();
            Callback::from(move |e: Event| value.set(e.target_unchecked_into::<HtmlInputElement>().value()))
        };

        let submit_passless = {
            ctx.link().callback_future(move |_| {
                let email = passless_email.clone();

                async move {
                    let resp = request::login_without_password(email.to_string()).await;

                    Msg::LoginPasswordlessResponse(resp.ok())
                }
            })
        };


        let on_change_pass_email = {
            let value = pass_email.setter();
            Callback::from(move |e: Event| value.set(e.target_unchecked_into::<HtmlInputElement>().value()))
        };

        let on_change_pass_pass = {
            let value = pass_pass.setter();
            Callback::from(move |e: Event| value.set(e.target_unchecked_into::<HtmlInputElement>().value()))
        };

        let submit_pass = {
            ctx.link().callback_future(move |_| {
                let email = pass_email.clone();
                let pass = pass_pass.clone();

                async move {
                    let resp = request::login_with_password(email.to_string(), pass.to_string()).await;

                    Msg::LoginPasswordResponse(resp.ok())
                }
            })
        };


        html! {
            <div class="login-container">
                <div class="center-normal">
                    <div class="center-container">
                        <h2>{ "Passwordless Login" }</h2>
                        <div class="form-container">
                            <label for="email">{ "Email Address" }</label>
                            <input type="email" name="email" id="email" onchange={ on_change_passless_email } />

                            <input type="submit" value="Log in" class="button" onclick={ submit_passless } />
                        </div>

                        <h2>{ "Password Login" }</h2>
                        <div class="form-container">
                            <label for="email">{ "Email Address" }</label>
                            <input type="email" name="email" id="email" onchange={ on_change_pass_email } />

                            <label for="password">{ "Password" }</label>
                            <input type="password" name="password" id="password" onchange={ on_change_pass_pass } />

                            <input type="submit" value="Log in" class="button" onclick={ submit_pass } />
                        </div>
                    </div>
                </div>
            </div>
        }
    }
}