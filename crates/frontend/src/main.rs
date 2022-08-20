use std::{sync::{Arc, Mutex}, mem::MaybeUninit};

use common::{BookId, PersonId, api::{WrappingResponse, ApiErrorResponse}, component::popup::{Popup, PopupType}};
use common_local::{api, Member, FileId, LibraryId};
use lazy_static::lazy_static;
use services::open_websocket_conn;
use yew::{prelude::*, html::Scope};
use yew_router::prelude::*;

use components::NavbarModule;

mod util;
mod pages;
mod request;
mod services;
mod components;


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

        scope.send_future(async {
            Msg::LoadMemberSelf(request::get_member_self().await)
        });
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

    Update
}

struct Model {
    has_loaded_member: bool
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link().clone();
        MAIN_MODEL.with(move |v| *v.lock().unwrap() = MaybeUninit::new(scope));

        ctx.link()
        .send_future(async {
            open_websocket_conn();

            Msg::LoadMemberSelf(request::get_member_self().await)
        });

        Self {
            has_loaded_member: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadMemberSelf(resp) => {
                match resp.ok() {
                    Ok(resp) => {
                        *MEMBER_SELF.lock().unwrap() = resp.member;
                    }

                    Err(err) => display_error(err),
                }

                self.has_loaded_member = true;
            }

            Msg::Update => {}
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
                <BrowserRouter>
                    <NavbarModule />
                    {
                        if self.has_loaded_member {
                            html! {
                                <Switch<Route> render={ Switch::render(switch) } />
                            }
                        } else {
                            html! {
                                <div>
                                    <h1>{ "Loading..." }</h1>
                                </div>
                            }
                        }
                    }
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


#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
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

    #[at("/options")]
    Options,

    #[at("/setup")]
    Setup,

    #[at("/")]
    Dashboard
}


fn switch(route: &Route) -> Html {
    log::info!("{:?}", route);

    if !is_signed_in() && route != &Route::Setup {
        return html! { <pages::LoginPage /> };
    }

    match route.clone() {
        Route::Login => {
            html! { <pages::LoginPage /> }
        }

        Route::Logout => {
            html! { <pages::LogoutPage /> }
        }

        Route::ViewLibrary { library_id } => {
            html! { <pages::LibraryPage library_id={library_id}  /> }
        }

        Route::ViewBook { book_id } => {
            html! { <pages::MediaView id={book_id}  /> }
        }

        Route::ReadBook { book_id } => {
            html! { <pages::ReadingBook id={book_id}  /> }
        }

        Route::People => {
            html! { <pages::AuthorListPage /> }
        }

        Route::ViewPerson { person_id } => {
            html! { <pages::AuthorView id={person_id}  /> }
        }

        Route::Options => {
            html! { <pages::OptionsPage /> }
        }

        Route::Setup => {
            html! { <pages::SetupPage /> }
        }

        Route::Dashboard => {
            html! { <pages::HomePage /> }
        }
    }
}


fn main() {
    wasm_logger::init(wasm_logger::Config::default());

    log::debug!("starting...");

    yew::start_app::<Model>();
}