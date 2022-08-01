use std::sync::{Arc, Mutex};

use books_common::{api, Member, MetadataId, FileId, LibraryId};
use lazy_static::lazy_static;
use services::open_websocket_conn;
use yew::prelude::*;
use yew_router::prelude::*;

use components::NavbarModule;

mod util;
mod pages;
mod request;
mod services;
mod components;


lazy_static! {
    pub static ref MEMBER_SELF: Arc<Mutex<Option<Member>>> = Arc::new(Mutex::new(None));
}

pub fn get_member_self() -> Option<Member> {
    MEMBER_SELF.lock().unwrap().clone()
}

pub fn is_signed_in() -> bool {
    get_member_self().is_some()
}


enum Msg {
    LoadMemberSelf(Option<api::GetMemberSelfResponse>)
}

struct Model {
    has_loaded_member: bool
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
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
            Msg::LoadMemberSelf(opt_resp) => {
                if let Some(resp) = opt_resp {
                    *MEMBER_SELF.lock().unwrap() = resp.member;
                }

                self.has_loaded_member = true;
            }
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                <NavbarModule />
                {
                    if self.has_loaded_member {
                        html! {
                            <Switch<Route> render={Switch::render(switch)} />
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

    #[at("/view/:meta_id")]
    ViewMeta { meta_id: MetadataId },

    #[at("/read/:book_id")]
    ReadBook { book_id: FileId },

    #[at("/people")]
    People,

    #[at("/options")]
    Options,

    #[at("/setup")]
    Setup,

    #[at("/")]
    #[not_found]
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

        Route::ViewMeta { meta_id } => {
            html! { <pages::MediaView id={meta_id}  /> }
        }

        Route::ReadBook { book_id } => {
            html! { <pages::ReadingBook id={book_id}  /> }
        }

        Route::People => {
            html! { <pages::AuthorListPage /> }
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

    yew::start_app::<Model>();
}