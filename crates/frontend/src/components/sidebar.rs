use common::{component::popup::{PopupClose, button::ButtonWithPopup}, api::WrappingResponse};
use common_local::{api, LibraryColl};
use yew::{prelude::*, html::Scope};
use yew_router::{prelude::Link, scope_ext::RouterScopeExt};

use crate::{Route, request};

pub enum Msg {
    LibraryListResults(WrappingResponse<api::GetLibrariesResponse>),

    Ignore,
}

pub struct Sidebar {
    library_items: Option<Vec<LibraryColl>>,
}

impl Component for Sidebar {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link()
        .send_future(async move {
            Msg::LibraryListResults(request::get_libraries().await)
        });

        Self {
            library_items: None
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => return false,

            Msg::LibraryListResults(resp) => {
                match resp.ok() {
                    Ok(resp) => self.library_items = Some(resp.items),
                    Err(err) => crate::display_error(err),
                }
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if let Some(items) = self.library_items.as_deref() {
            html! {
                <div class="sidebar-container">
                    { for items.iter().map(|item| Self::render_sidebar_library_item(item, ctx.link())) }
                </div>
            }
        } else {
            html! {
                <div class="sidebar-container">
                    <h1>{ "..." }</h1>
                </div>
            }
        }
    }
}

impl Sidebar {
    fn render_sidebar_library_item(item: &LibraryColl, scope: &Scope<Self>) -> Html {
        let library_id = item.id;

        let to = Route::ViewLibrary { library_id };
        let cr = scope.route::<Route>().unwrap();


        html! {
            <Link<Route> {to} classes={ classes!("sidebar-item", "library", (cr == to).then(|| "active")) }>
                <span class="title">{ item.name.clone() }</span>
                <div class="options">
                    <ButtonWithPopup class="menu-list">
                        <PopupClose class="menu-item" onclick={ scope.callback_future(move |e: MouseEvent| {
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

                        <PopupClose class="menu-item" onclick={ scope.callback_future(move |e: MouseEvent| {
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
                    </ButtonWithPopup>
                </div>
            </Link<Route>>
        }
    }
}