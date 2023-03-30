use common::component::PopupType;
use common::component::select::{SelectModule, SelectItem};
use common::{api::WrappingResponse, component::Popup};
use common_local::{api, GroupPermissions, LibraryAccessPreferences};
use gloo_utils::window;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::request;

pub enum Msg {
    // Request Results
    MembersResults(Box<WrappingResponse<api::ApiGetMembersListResponse>>),

    RequestUpdateOptions(api::UpdateMember),
    InviteMember { email: String },

    OpenMemberPopup(usize),
    CloseMemberPopup,

    Ignore,
}

pub struct AdminMembersPage {
    resp: Option<api::ApiGetMembersListResponse>,
    visible_popup: Option<usize>,
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

            Msg::OpenMemberPopup(id) => {
                self.visible_popup = Some(id);
            }

            Msg::CloseMemberPopup => {
                self.visible_popup = None;
            }

            Msg::Ignore => (),
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let render = if let Some(resp) = self.resp.as_ref() {
            let input_ref = NodeRef::default();

            let (members_invited, members_accepted): (Vec<_>, Vec<_>) =
                resp.items.iter().partition(|v| v.type_of.is_invited());

            html! {
                // We use a empty div to prevent the buttons' widths from fully expanding.
                <div class="horizontal-center">
                    <div class="section-invite">
                        <h3>{ "Invite Someone" }</h3>

                        <div class="input-group mb-3">
                            <input class="form-control" ref={ input_ref.clone() } type="email" placeholder="Email Address" />
                            <button class="btn btn-primary" onclick={ ctx.link().callback(move |_| {
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
                    </div>

                    <table class="table table-dark table-striped">
                        <thead>
                            <tr>
                                <td colspan="4">
                                    <h4>{ "Members" }</h4>
                                </td>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                for members_accepted.iter()
                                    .enumerate()
                                    .map(|(idx, v)| {
                                        let member_id = v.id;

                                        html! {
                                            <tr class="list-item">
                                                <td>
                                                    <span class="label" title="Permission Grouping">{ format!("{:?}", v.permissions.group) }</span>
                                                </td>
                                                <td>
                                                    <span class="title" title={ v.email.clone() }>{ v.name.clone() }</span>
                                                </td>

                                                {
                                                    if v.permissions.is_owner() {
                                                        html! {
                                                            <>
                                                                <td></td>
                                                                <td></td>
                                                            </>
                                                        }
                                                    } else {
                                                        html! {
                                                            <>
                                                                <td>
                                                                    <button class="btn btn-danger btn-sm" onclick={ ctx.link().callback(move|_| {
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
                                                                <td>
                                                                    <button
                                                                        class="btn btn-warning btn-sm"
                                                                        onclick={ ctx.link().callback(move |_| Msg::OpenMemberPopup(idx)) }
                                                                    >{ "⚙️" }</button>
                                                                </td>
                                                            </>
                                                        }
                                                    }
                                                }
                                            </tr>
                                        }
                                    })
                            }
                        </tbody>
                    </table>

                    <table class="table table-dark table-striped">
                        <thead>
                            <tr>
                                <td colspan="3">
                                    <h4>{ "Pending Invitations" }</h4>
                                </td>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                for members_invited.iter()
                                    .map(|v| {
                                        let member_id = v.id;

                                        html! {
                                            <tr>
                                                <td>
                                                    <span class="label" title="Permission Grouping">{ format!("{:?}", v.permissions.group) }</span>
                                                </td>
                                                <td>
                                                    <span class="title" title={ v.email.clone() }>{ v.name.clone() }</span>
                                                </td>

                                                {
                                                    if v.permissions.is_owner() {
                                                        html! {
                                                            <td></td>
                                                        }
                                                    } else {
                                                        html! {
                                                            <td>
                                                                <button class="btn btn-danger btn-sm" onclick={ ctx.link().callback(move|_| {
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
            <div class="view-container admin-members">
                { render }

                {
                    if let Some(member_id) = self.visible_popup {
                        self.render_popup(member_id, ctx)
                    } else {
                        html! {}
                    }
                }
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

impl AdminMembersPage {
    pub fn render_popup(&self, member_index: usize, ctx: &Context<Self>) -> Html {
        let Some(resp) = self.resp.as_ref() else {
            return html! {};
        };

        let member = &resp.items[member_index];

        let libraries = crate::get_libraries();

        // TODO: Should not be baked in. We should have to call a endpoint get the person's (proper) preferences.
        let acc_libraries = match member.parse_preferences() {
            Ok(Some(v)) => v.library_access.get_accessible_libraries(&libraries),
            Ok(None) => LibraryAccessPreferences::default().get_accessible_libraries(&libraries),
            Err(e) => return html! {
                <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::CloseMemberPopup) }>
                    <h2>{ "Parse Preferences Error: " }{ e }</h2>
                </Popup>
            },
        };

        html! {
            <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::CloseMemberPopup) }>
                <div class="modal-header">
                    <h5 class="modal-title">{ "Edit Member" }</h5>
                    <button
                        type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"
                        onclick={ ctx.link().callback(|_| Msg::CloseMemberPopup) }
                    ></button>
                </div>
                <div class="modal-body">
                    <h3>{ "Permissions" }</h3>

                    <div class="mb-3">
                        <label class="form-label">{ "Group" }</label>
                        <SelectModule<GroupPermissions> class="form-select" default={ member.permissions.group }>
                            <SelectItem<GroupPermissions> value={ GroupPermissions::OWNER } name="Owner" />
                            <SelectItem<GroupPermissions> value={ GroupPermissions::BASIC } name="Basic" />
                            <SelectItem<GroupPermissions> value={ GroupPermissions::GUEST } name="Guest" />
                        </SelectModule<GroupPermissions>>
                    </div>

                    <div class="mb-3">
                        <label class="form-label">{ "Library Access" }</label>
                        {
                            for libraries.iter().map(|lib| html! {
                                <div class="form-check">
                                    <input class="form-check-input" type="checkbox" checked={ acc_libraries.iter().any(|v| lib.id == v.id) } />
                                    <label class="form-check-label">{ lib.name.clone() }</label>
                                </div>
                            })
                        }
                    </div>
                </div>

                <div class="modal-footer">
                    <button type="button" class="btn btn-secondary" onclick={ ctx.link().callback(|_| Msg::CloseMemberPopup) }>{ "Close" }</button>
                    <button type="button" class="btn btn-primary">{ "Save changes" }</button>
                </div>
            </Popup>
        }
    }
}