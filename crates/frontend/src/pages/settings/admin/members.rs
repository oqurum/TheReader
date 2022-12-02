use common::MemberId;
use common::api::WrappingResponse;
use common_local::api;
use gloo_utils::window;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::request;
use crate::pages::settings::SettingsSidebar;

pub enum Msg {
    // Request Results
    MembersResults(Box<WrappingResponse<api::ApiGetMembersListResponse>>),

    // Events
    DisplayPopup(usize, MemberId),
    ClosePopup,

    RequestUpdateOptions(api::UpdateMember),
    InviteMember {
        email: String,
    },

    Ignore,
}

pub struct AdminMembersPage {
    resp: Option<api::ApiGetMembersListResponse>,
    visible_popup: Option<(usize, MemberId)>,
}

impl Component for AdminMembersPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            resp: None,
            visible_popup: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::MembersResults(resp) => match resp.ok() {
                Ok(resp) => {
                    self.resp = Some(resp);
                    self.visible_popup = None;
                }
                Err(err) => crate::display_error(err),
            },

            Msg::DisplayPopup(popup, index) => {
                self.visible_popup = Some((popup, index));
            }

            Msg::ClosePopup => {
                self.visible_popup = None;
            }

            Msg::RequestUpdateOptions(options) => {
                ctx.link().send_future(async move {
                    request::update_member(options).await;

                    Msg::MembersResults(Box::new(request::get_members().await))
                });
            }

            Msg::InviteMember { email } => {
                ctx.link().send_future(async move {
                    request::update_member(api::UpdateMember::Invite { email }).await;

                    Msg::MembersResults(Box::new(request::get_members().await))
                });
            }

            Msg::Ignore => (),
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let render = if let Some(resp) = self.resp.as_ref() {
            let input_ref = NodeRef::default();

            let (members_invited, members_accepted): (Vec<_>, Vec<_>) = resp.items.iter().partition(|v| v.type_of.is_invited());

            html! {
                // We use a empty div to prevent the buttons' widths from fully expanding.
                <div class="horizontal-center">
                    <div class="section-invite">
                        <h3>{ "Invite Someone" }</h3>

                        <input ref={ input_ref.clone() } type="email" placeholder="Email Address" />
                        <button class="green" onclick={ ctx.link().callback(move |_| {
                            let input = input_ref.get().unwrap().unchecked_into::<HtmlInputElement>();

                            if input.check_validity() {
                                Msg::InviteMember {
                                    email: input_ref.get().unwrap().unchecked_into::<HtmlInputElement>().value(),
                                }
                            } else {
                                Msg::Ignore
                            }
                        }) }>{ "Invite" }</button>
                    </div>

                    <h3>{ "Members" }</h3>
                    <table class="members-list">
                        <tbody>
                            {
                                for members_accepted.iter()
                                    .map(|v| {
                                        let member_id = v.id;

                                        html! {
                                            <tr class="list-item">
                                                <td class="cell-label">
                                                    <span class="label" title="Permission Grouping">{ format!("{:?}", v.permissions.group) }</span>
                                                </td>
                                                <td class="cell-title">
                                                    <span class="title" title={ v.email.clone() }>{ v.name.clone() }</span>
                                                </td>

                                                {
                                                    if v.permissions.is_owner() {
                                                        html! {
                                                            <td class="cell-button"></td>
                                                        }
                                                    } else {
                                                        html! {
                                                            <td class="cell-button">
                                                                <button class="slim red" onclick={ ctx.link().callback(move|_| {
                                                                    if window().confirm_with_message("Are you sure you want to delete this?").unwrap_throw() {
                                                                        Msg::RequestUpdateOptions(
                                                                            api::UpdateMember::Delete {
                                                                                id: member_id
                                                                            }
                                                                        )
                                                                    } else {
                                                                        Msg::Ignore
                                                                    }
                                                                }) }>{ "Remove Access" }</button>
                                                            </td>
                                                        }
                                                    }
                                                }
                                            </tr>
                                        }
                                    })
                            }
                            <tr class="list-item">
                                <td colspan="3">
                                    <h5>{ "Pending an Invitation" }</h5>
                                </td>
                            </tr>
                            {
                                for members_invited.iter()
                                    .map(|v| {
                                        let member_id = v.id;

                                        html! {
                                            <tr class="list-item">
                                                <td class="cell-label">
                                                    <span class="label" title="Permission Grouping">{ format!("{:?}", v.permissions.group) }</span>
                                                </td>
                                                <td class="cell-title">
                                                    <span class="title" title={ v.email.clone() }>{ v.name.clone() }</span>
                                                </td>

                                                {
                                                    if v.permissions.is_owner() {
                                                        html! {
                                                            <td class="cell-button"></td>
                                                        }
                                                    } else {
                                                        html! {
                                                            <td class="cell-button">
                                                                <button class="slim red" onclick={ ctx.link().callback(move|_| {
                                                                    if window().confirm_with_message("Are you sure you want to delete this?").unwrap_throw() {
                                                                        Msg::RequestUpdateOptions(
                                                                            api::UpdateMember::Delete {
                                                                                id: member_id
                                                                            }
                                                                        )
                                                                    } else {
                                                                        Msg::Ignore
                                                                    }
                                                                }) }>{ "Cancel Invite" }</button>
                                                            </td>
                                                        }
                                                    }
                                                }
                                            </tr>
                                        }
                                    })
                            }
                        </tbody>
                    </table>
                </div>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        };

        html! {
            <div class="outer-view-container">
                <SettingsSidebar />
                <div class="view-container admin-members">
                    { render }
                </div>
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link()
                .send_future(async { Msg::MembersResults(Box::new(request::get_members().await)) });
        }
    }
}
