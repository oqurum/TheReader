use common::api::WrappingResponse;
use common_local::api;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::pages::settings::SettingsSidebar;
use crate::request;

pub enum Msg {
    // Request Results
    OptionsResults(Box<WrappingResponse<api::GetOptionsResponse>>),

    // Events
    RequestUpdateOptions(bool, api::ModifyOptionsBody),
}

pub struct AdminMyServerPage {
    resp: Option<api::GetOptionsResponse>,
}

impl Component for AdminMyServerPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self { resp: None }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::OptionsResults(resp) => match resp.ok() {
                Ok(resp) => self.resp = Some(resp),
                Err(err) => crate::display_error(err),
            },

            Msg::RequestUpdateOptions(is_adding, options) => {
                ctx.link().send_future(async move {
                    if is_adding {
                        request::update_options_add(options).await;
                    } else {
                        request::update_options_remove(options).await;
                    }

                    Msg::OptionsResults(Box::new(request::get_options().await))
                });
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let render = if let Some(resp) = self.resp.as_ref() {
            let config = resp.config.as_ref().unwrap();

            html! {
                <>
                    <h4>{ "External Metadata Server" }</h4>
                    {
                        if config.libby.token.is_some() {
                            html! {
                                <div class="form-container shrink-width-to-content">
                                    <span class="label green">
                                        { "Metadata Server Link is Setup: " }
                                        <a href={ config.libby.url.clone() }>{ config.libby.url.clone() }</a>
                                    </span>

                                    <label>{ "Search Return Type" }</label>
                                    <select onchange={ ctx.link().callback(|v: Event| {
                                        Msg::RequestUpdateOptions(
                                            true,
                                            api::ModifyOptionsBody {
                                                libby_public_search: Some(v.target_unchecked_into::<HtmlSelectElement>().selected_index() == 0),
                                                .. Default::default()
                                            }
                                        )
                                    }) }>
                                        <option selected={ config.libby.public_only }>{ "Public Only" }</option>
                                        <option selected={ !config.libby.public_only }>{ "All" }</option>
                                    </select>
                                </div>
                            }
                        } else {
                            html! {
                                <>
                                    <span class="label red">
                                        { "Click below to setup the Metadata Server Link for " }
                                        <b><a href={ config.libby.url.clone() }>{ config.libby.url.clone() }</a></b>
                                    </span>
                                    <br />
                                    <form action="/api/setup/agent" method="POST">
                                        <button class="green" type="submit">{ "Link" }</button>
                                    </form>
                                </>
                            }
                        }
                    }
                </>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        };

        html! {
            <div class="outer-view-container">
                <SettingsSidebar />
                <div class="view-container">
                    { render }
                </div>
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link()
                .send_future(async { Msg::OptionsResults(Box::new(request::get_options().await)) });
        }
    }
}
