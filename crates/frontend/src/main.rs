#![allow(clippy::let_unit_value, clippy::type_complexity)]

use std::{
    mem::MaybeUninit,
    rc::Rc,
    sync::{Arc, Mutex},
};

use common::{
    api::{ApiErrorResponse, WrappingResponse},
    component::popup::{Popup, PopupType},
    BookId, PersonId,
};
use common_local::{api, FileId, LibraryId, Member, CollectionId};
use lazy_static::lazy_static;
use services::open_websocket_conn;
use yew::{html::Scope, prelude::*};
use yew_router::prelude::*;

use components::NavbarModule;

mod components;
mod pages;
mod request;
mod services;
mod util;

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub is_navbar_visible: bool,
    pub update_nav_visibility: Callback<bool>,
}

lazy_static! {
    pub static ref MEMBER_SELF: Arc<Mutex<Option<Member>>> = Arc::new(Mutex::new(None));
    static ref ERROR_POPUP: Arc<Mutex<Option<ApiErrorResponse>>> = Arc::new(Mutex::new(None));
}

thread_local! {
    static MAIN_MODEL: Arc<Mutex<MaybeUninit<Scope<Model>>>> = Arc::new(Mutex::new(MaybeUninit::uninit()));
}

pub fn get_member_self() -> Option<Member> {
    MEMBER_SELF.lock().unwrap().clone()
}

pub fn is_signed_in() -> bool {
    get_member_self().is_some()
}

pub fn request_member_self() {
    MAIN_MODEL.with(|v| unsafe {
        let lock = v.lock().unwrap();

        let scope = lock.assume_init_ref();

        scope.send_future(async { Msg::LoadMemberSelf(request::get_member_self().await) });
    });
}

pub fn display_error(value: ApiErrorResponse) {
    {
        *ERROR_POPUP.lock().unwrap() = Some(value);
    }

    MAIN_MODEL.with(|v| unsafe {
        let lock = v.lock().unwrap();

        let scope = lock.assume_init_ref();

        scope.send_message(Msg::Update);
    });
}

fn remove_error() {
    {
        *ERROR_POPUP.lock().unwrap() = None;
    }

    MAIN_MODEL.with(|v| unsafe {
        let lock = v.lock().unwrap();

        let scope = lock.assume_init_ref();

        scope.send_message(Msg::Update);
    });
}

enum Msg {
    LoadMemberSelf(WrappingResponse<api::GetMemberSelfResponse>),

    UpdateNavVis(bool),

    Update,
}

struct Model {
    state: Rc<AppState>,

    has_loaded_member: bool,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link().clone();
        MAIN_MODEL.with(move |v| *v.lock().unwrap() = MaybeUninit::new(scope));

        ctx.link()
            .send_future(async { Msg::LoadMemberSelf(request::get_member_self().await) });

        Self {
            state: Rc::new(AppState {
                is_navbar_visible: true,
                update_nav_visibility: ctx.link().callback(Msg::UpdateNavVis),
            }),
            has_loaded_member: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadMemberSelf(resp) => {
                if let WrappingResponse::Resp(resp) = resp {
                    if resp.member.is_some() {
                        open_websocket_conn();
                    }

                    *MEMBER_SELF.lock().unwrap() = resp.member;
                }

                self.has_loaded_member = true;
            }

            Msg::UpdateNavVis(value) => {
                Rc::make_mut(&mut self.state).is_navbar_visible = value;
            }

            Msg::Update => (),
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
                <BrowserRouter>
                    <ContextProvider<Rc<AppState>> context={ self.state.clone() }>
                        <NavbarModule visible={ self.state.is_navbar_visible } />
                        {
                            if self.has_loaded_member {
                                html! {
                                    <Switch<BaseRoute> render={ Switch::render(switch_base) } />
                                }
                            } else {
                                html! {
                                    <div>
                                        <h1>{ "Loading..." }</h1>
                                    </div>
                                }
                            }
                        }
                    </ContextProvider<Rc<AppState>>>
                </BrowserRouter>

                {
                    if let Some(error) = ERROR_POPUP.lock().unwrap().as_ref() {
                        html! {
                            <Popup
                                type_of={ PopupType::FullOverlay }
                                on_close={ Callback::from(|_| remove_error()) }
                            >
                                <p>{ error.description.clone() }</p>
                            </Popup>
                        }
                    } else {
                        html! {}
                    }
                }
            </>
        }
    }
}

#[derive(Routable, PartialEq, Eq, Clone, Debug)]
pub enum BaseRoute {
    #[at("/login")]
    Login,

    #[at("/logout")]
    Logout,

    #[at("/library/:library_id")]
    ViewLibrary { library_id: LibraryId },

    #[at("/view/:book_id")]
    ViewBook { book_id: BookId },

    #[at("/read/:book_id")]
    ReadBook { book_id: FileId },

    #[at("/people")]
    People,

    #[at("/person/:person_id")]
    ViewPerson { person_id: PersonId },

    #[at("/collections")]
    Collections,

    #[at("/collection/:id")]
    ViewCollection { id: CollectionId },

    #[at("/settings/*")]
    Settings,

    #[at("/setup")]
    Setup,

    #[at("/")]
    Dashboard,
}

fn switch_base(route: &BaseRoute) -> Html {
    log::info!("{:?}", route);

    if !is_signed_in() && route != &BaseRoute::Setup {
        return html! { <pages::LoginPage /> };
    }

    match route.clone() {
        BaseRoute::Login => {
            html! { <pages::LoginPage /> }
        }

        BaseRoute::Logout => {
            html! { <pages::LogoutPage /> }
        }

        BaseRoute::ViewLibrary { library_id } => {
            html! { <pages::LibraryPage library_id={library_id} /> }
        }

        BaseRoute::ViewBook { book_id } => {
            html! { <pages::MediaView id={book_id} /> }
        }

        BaseRoute::ReadBook { book_id } => {
            html! { <pages::ReadingBook id={book_id} /> }
        }

        BaseRoute::People => {
            html! { <pages::AuthorListPage /> }
        }

        BaseRoute::ViewPerson { person_id } => {
            html! { <pages::AuthorView id={person_id} /> }
        }

        BaseRoute::Collections => {
            html! { <pages::CollectionListPage /> }
        }

        BaseRoute::ViewCollection { id } => {
            html! { <pages::CollectionItemPage {id} /> }
        }

        BaseRoute::Settings => {
            html! { <Switch<pages::settings::SettingsRoute> render={Switch::render(pages::settings::switch_settings)} /> }
        }

        BaseRoute::Setup => {
            html! { <pages::SetupPage /> }
        }

        BaseRoute::Dashboard => {
            html! { <pages::HomePage /> }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());

    log::debug!("starting...");

    yew::start_app::<Model>();
}
