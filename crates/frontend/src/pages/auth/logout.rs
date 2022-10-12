use gloo_timers::callback::Timeout;
use gloo_utils::window;
use yew::prelude::*;

use crate::is_signed_in;

pub struct LogoutPage;

impl Component for LogoutPage {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        Timeout::new(2_000, || {
            let _ = window().location().set_href("/auth/logout");
        })
        .forget();

        if is_signed_in() {
            html! {
                <div class="login-container">
                    <div class="center-normal">
                        <div class="center-container">
                            <h2>{ "Logging Out..." }</h2>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="login-container">
                    <div class="center-normal">
                        <div class="center-container">
                            <h2>{ "Successfully Logged Out" }</h2>
                        </div>
                    </div>
                </div>
            }
        }
    }
}
