use std::rc::Rc;

use common::component::popup::{button::ButtonWithPopup, Popup, PopupClose, PopupType};
use common_local::{api, LibraryColl, LibraryId};
use yew::{html::Scope, prelude::*};
use yew_router::{
    prelude::{Link, Location},
    scope_ext::{RouterScopeExt, LocationHandle},
};

use crate::{
    components::edit::library::LibraryEdit, pages::settings::SettingsRoute, request, BaseRoute, AppState,
};

use super::OwnerBarrier;

#[derive(Properties, PartialEq, Eq)]
pub struct Props {
    pub visible: bool,
}

pub enum Msg {
    EditLibrary(LibraryId),
    HideEdit,

    LocationChange(Location),

    Ignore,

    ContextChanged(Rc<AppState>),
}

pub struct Sidebar {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    library_editing: Option<LibraryId>,

    _location_handle: Option<LocationHandle>,

    viewing: Viewing,
}

impl Component for Sidebar {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");

        Self {
            state,
            _listener,

            viewing: Viewing::get_from_route(ctx.link()),

            library_editing: None,

            _location_handle: ctx
                .link()
                .add_location_listener(ctx.link().callback(Msg::LocationChange)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => self.state = state,

            Msg::Ignore => return false,

            Msg::LocationChange(_nav) => {
                self.viewing = Viewing::get_from_route(ctx.link());
                debug!("-- {:?}", self.viewing);
            }

            Msg::HideEdit => {
                self.library_editing = None;
            }

            Msg::EditLibrary(id) => {
                self.library_editing = Some(id);
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if !ctx.props().visible {
            return html! {};
        }

        html! {
            <div class="sidebar-container d-sm-flex flex-column flex-shrink-0 p-2 text-bg-dark collapse collapse-horizontal" id="sidebarToggle">
                { self.render(&self.state.libraries, ctx) }
            </div>
        }
    }
}

impl Sidebar {
    fn render(&self, items: &[LibraryColl], ctx: &Context<Self>) -> Html {
        match self.viewing {
            Viewing::Main => html! {
                <>
                    <Link<BaseRoute> to={ BaseRoute::Dashboard } classes={ "d-flex align-items-center mb-3 me-md-auto text-white text-decoration-none" }>
                        <span class="fs-4">{ "Reader" }</span>
                    </Link<BaseRoute>>

                    <hr />

                    <ul class="nav nav-pills flex-column">
                        <li class="nav-item">
                            <Link<BaseRoute> to={ BaseRoute::Collections } classes={ classes!("nav-link", "text-white") }>
                                // <span class="material-icons bi pe-none me-2">{ "library_books" }</span>
                                { "My Collections" }
                            </Link<BaseRoute>>
                        </li>
                    </ul>

                    <hr />

                    <ul class="nav nav-pills flex-column mb-auto">
                        { for items.iter().map(|item| Self::render_sidebar_library_item(item, ctx.link())) }
                    </ul>

                    {
                        if let Some(id) = self.library_editing {
                            html! {
                                <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::HideEdit) }>
                                    <LibraryEdit {id} />
                                </Popup>
                            }
                        } else {
                            html! {}
                        }
                    }
                </>
            },

            Viewing::Settings => {
                const ADMIN_LOCATIONS: [(&str, SettingsRoute); 4] = [
                    ("Tasks", SettingsRoute::AdminTasks),
                    ("Members", SettingsRoute::AdminMembers),
                    ("My Server", SettingsRoute::AdminMyServer),
                    ("Libraries", SettingsRoute::AdminLibraries),
                ];

                const MEMBERS_LOCATIONS: [(&str, SettingsRoute); 1] = [
                    ("General", SettingsRoute::MemberGeneral),
                ];

                let cr = ctx.link().route::<SettingsRoute>().unwrap();

                html! {
                    <>
                        <Link<BaseRoute> to={ BaseRoute::Dashboard } classes={ "d-flex align-items-center mb-3 mb-md-0 me-md-auto text-white text-decoration-none" }>
                            <span class="fs-4">{ "Reader" }</span>
                        </Link<BaseRoute>>

                        <hr />

                        <OwnerBarrier>
                            <div class="sidebar-item">
                                <h3>
                                    { "Admin" }
                                </h3>
                            </div>

                            <ul class="nav nav-pills flex-column">
                                { for ADMIN_LOCATIONS.iter().map(|&(title, route)| html! {
                                    <li class="nav-item">
                                        <Link<SettingsRoute> to={route} classes={ classes!("nav-link", (cr == route).then_some("active")) }>
                                            <span class="title">{ title }</span>
                                        </Link<SettingsRoute>>
                                    </li>
                                }) }
                            </ul>
                        </OwnerBarrier>

                        <div class="sidebar-item">
                            <h3>
                                { "Members" }
                            </h3>
                        </div>

                        <ul class="nav nav-pills flex-column">
                            { for MEMBERS_LOCATIONS.iter().map(|&(title, route)| html! {
                                <li class="nav-item">
                                    <Link<SettingsRoute> to={route} classes={ classes!("nav-link", (cr == route).then_some("active")) }>
                                        <span class="title">{ title }</span>
                                    </Link<SettingsRoute>>
                                </li>
                            }) }
                        </ul>
                    </>
                }
            }
        }
    }

    fn render_sidebar_library_item(item: &LibraryColl, scope: &Scope<Self>) -> Html {
        let library_id = item.id;

        let to = BaseRoute::ViewLibrary { id: library_id };
        // let cr = scope.route::<BaseRoute>().unwrap();

        html! {
            <li class="nav-item">
                <Link<BaseRoute> {to} classes={ classes!("nav-link", "text-white", "library"/*, (cr == to).then_some("active")*/) }>
                    { item.name.clone() }
                </Link<BaseRoute>>

                <OwnerBarrier>
                    <div class="options">
                        <ButtonWithPopup class="menu-list">
                            <PopupClose class="dropdown-item" onclick={ scope.callback_future(move |e: MouseEvent| {
                                e.prevent_default();

                                async move {
                                    request::run_task(api::RunTaskBody {
                                        run_metadata: Some(library_id),

                                        .. Default::default()
                                    }).await;

                                    Msg::Ignore
                                }
                            }) }>
                                { "Refresh All Metadata" }
                            </PopupClose>

                            <PopupClose class="dropdown-item" onclick={ scope.callback_future(move |e: MouseEvent| {
                                e.prevent_default();

                                async move {
                                    request::run_task(api::RunTaskBody {
                                        run_search: Some(library_id),

                                        .. Default::default()
                                    }).await;

                                    Msg::Ignore
                                }
                            }) }>
                                { "Library Scan" }
                            </PopupClose>

                            <PopupClose class="dropdown-item" onclick={ scope.callback(move |e: MouseEvent| {
                                e.prevent_default();
                                e.stop_propagation();

                                Msg::EditLibrary(library_id)
                            }) }>
                                { "Edit Library" }
                            </PopupClose>
                        </ButtonWithPopup>
                    </div>
                </OwnerBarrier>
            </li>
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Viewing {
    Main,
    Settings,
}

impl Viewing {
    pub fn get_from_route(scope: &Scope<Sidebar>) -> Self {
        let route = scope.route::<BaseRoute>().unwrap();

        if matches!(route, BaseRoute::Settings) {
            Self::Settings
        } else {
            Self::Main
        }
    }
}
