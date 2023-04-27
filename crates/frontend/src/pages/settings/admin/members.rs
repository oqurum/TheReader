use std::rc::Rc;

use common::component::select::{SelectItem, SelectModule};
use common::component::PopupType;
use common::MemberId;
use common::{api::WrappingResponse, component::Popup};
use common_local::{api, GroupPermissions, LibraryAccess, Member, MemberUpdate, Permissions};
use gloo_utils::window;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{request, AppState};

pub enum Msg {
    // Request Results
    MemberListResults(Box<WrappingResponse<api::ApiGetMembersListResponse>>),
    MemberResults(Box<WrappingResponse<api::ApiGetMemberSelfResponse>>),

    RequestUpdateOptions(api::UpdateMember),
    InviteMember {
        email: String,
    },

    UpdateMember {
        id: MemberId,
        update: Box<MemberUpdate>,
    },
    OpenMemberPopup(usize),
    CloseMemberPopup,

    Ignore,

    ContextChanged(Rc<AppState>),
}

pub struct AdminMembersPage {
    resp: Option<api::ApiGetMembersListResponse>,
    visible_popup: Option<usize>,

    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,
}

impl Component for AdminMembersPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");

        Self {
            state,
            _listener,

            resp: None,
            visible_popup: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => self.state = state,

            Msg::MemberListResults(resp) => match resp.ok() {
                Ok(resp) => {
                    self.resp = Some(resp);
                    self.visible_popup = None;
                }

                Err(err) => crate::display_error(err),
            },

            Msg::MemberResults(resp) => match resp.ok() {
                Ok(resp) => {
                    if let Some((members, updated_member)) = self.resp.as_mut().zip(resp.member) {
                        if let Some(member) =
                            members.items.iter_mut().find(|v| v.id == updated_member.id)
                        {
                            *member = updated_member;
                        }
                    }
                }

                Err(err) => crate::display_error(err),
            },

            Msg::RequestUpdateOptions(options) => {
                ctx.link().send_future(async move {
                    request::update_member(options).await;

                    Msg::MemberListResults(Box::new(request::get_members().await))
                });
            }

            Msg::InviteMember { email } => {
                ctx.link().send_future(async move {
                    request::update_member(api::UpdateMember::Invite { email }).await;

                    Msg::MemberListResults(Box::new(request::get_members().await))
                });
            }

            Msg::OpenMemberPopup(id) => {
                self.visible_popup = Some(id);
            }

            Msg::CloseMemberPopup => {
                self.visible_popup = None;
            }

            Msg::UpdateMember { id, update } => {
                ctx.link().send_future(async move {
                    let resp = request::update_member_id(id, *update).await;

                    Msg::MemberResults(Box::new(resp))
                });
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
                                                    if self.state.member.as_ref().map(|v| v.id) == Some(member_id) {
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
            ctx.link().send_future(async {
                Msg::MemberListResults(Box::new(request::get_members().await))
            });
        }
    }
}

impl AdminMembersPage {
    pub fn render_popup(&self, member_index: usize, ctx: &Context<Self>) -> Html {
        let Some(resp) = self.resp.as_ref() else {
            return html! {};
        };

        html! {
            <PopupMemberEdit member={ resp.items[member_index].clone() } ctx={ ctx.link().callback(|v| v) } />
        }
    }
}

#[derive(PartialEq, Properties)]
struct PopupMemberEditProps {
    pub member: Member,
    pub ctx: Callback<Msg>,
}

#[function_component(PopupMemberEdit)]
fn _popup_member_edit(props: &PopupMemberEditProps) -> Html {
    let PopupMemberEditProps { member, ctx } = props;

    // TODO: Remove fill_with_member. It should be empty. Added temporarily to ensure the permissions is correct.
    let updating = use_mut_ref(|| MemberUpdate::fill_with_member(member));

    let on_select_module = {
        let updating = updating.clone();

        Callback::from(move |v| {
            let mut write = updating.borrow_mut();
            write
                .permissions
                .get_or_insert_with(Permissions::basic)
                .group = v;
        })
    };

    let on_save = {
        let updating = updating.clone();
        let member_id = member.id;

        ctx.reform(move |_| Msg::UpdateMember {
            id: member_id,
            update: Box::new(updating.borrow().clone()),
        })
    };

    let libraries = crate::get_libraries();

    let library_ref = updating.borrow();

    let acc_libraries = if let Some(v) = library_ref.library_access.as_ref() {
        v.get_accessible_libraries(&libraries)
    } else {
        // TODO: Should not be baked in. We should have to call a endpoint get the person's (proper) preferences.
        match member.parse_library_access_or_default() {
            Ok(v) => v.get_accessible_libraries(&libraries),
            Err(e) => {
                return html! {
                    <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.reform(|_| Msg::CloseMemberPopup) }>
                        <h2>{ "Parse Preferences Error: " }{ e }</h2>
                    </Popup>
                }
            }
        }
    };

    html! {
        <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.reform(|_| Msg::CloseMemberPopup) }>
            <div class="modal-header">
                <h5 class="modal-title">{ "Edit Member" }</h5>
                <button
                    type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"
                    onclick={ ctx.reform(|_| Msg::CloseMemberPopup) }
                ></button>
            </div>
            <div class="modal-body">
                <div class="mb-3">
                    <label class="form-label">{ "Permissions Group" }</label>
                    <SelectModule<GroupPermissions>
                        class="form-select"
                        default={ library_ref.permissions.as_ref().unwrap_or(&member.permissions).group }
                        onselect={ on_select_module }
                    >
                        <SelectItem<GroupPermissions> value={ GroupPermissions::OWNER } name="Owner" />
                        <SelectItem<GroupPermissions> value={ GroupPermissions::BASIC } name="Basic" />
                        <SelectItem<GroupPermissions> value={ GroupPermissions::GUEST } name="Guest" />
                    </SelectModule<GroupPermissions>>
                </div>

                <div class="mb-3">
                    <label class="form-label">{ "Library Access" }</label>
                    {
                        for libraries.iter().map(|lib| {
                            let updating = updating.clone();

                            let lib_id = lib.id;
                            let checked = acc_libraries.iter().any(|v| lib.id == v.id);

                            html! {
                                <div class="form-check">
                                    <input
                                        class="form-check-input"
                                        type="checkbox"
                                        {checked}
                                        onclick={ Callback::from(move |_| {
                                            let libraries = crate::get_libraries();

                                            updating.borrow_mut().library_access
                                                .get_or_insert_with(LibraryAccess::default)
                                                .set_viewable(lib_id, !checked, &libraries);
                                        }) }
                                    />
                                    <label class="form-check-label">{ lib.name.clone() }</label>
                                </div>
                            }
                        })
                    }
                </div>
            </div>

            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" onclick={ ctx.reform(|_| Msg::CloseMemberPopup) }>{ "Close" }</button>
                <button type="button" class="btn btn-primary" onclick={ on_save }>{ "Save changes" }</button>
            </div>
        </Popup>
    }
}
