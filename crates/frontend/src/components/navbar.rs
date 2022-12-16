use std::sync::{Arc, Mutex};

use common::{api::WrappingResponse, util::does_parent_contain_class};
use common_local::{api::GetBookListResponse, filter::FilterContainer, ThumbnailStoreExt};
use gloo_utils::body;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::components::Link;

use crate::{get_member_self, pages::settings::SettingsRoute, request, BaseRoute, components::BookListItemInfo};

#[derive(PartialEq, Eq, Properties)]
pub struct Property {
    pub visible: bool,
}

pub enum Msg {
    Close,
    SearchFor(String),
    SearchResults(WrappingResponse<GetBookListResponse>),
}

pub struct NavbarModule {
    left_items: Vec<(BaseRoute, DisplayType)>,
    right_items: Vec<(BaseRoute, DisplayType)>,

    search_results: Option<GetBookListResponse>,
    #[allow(clippy::type_complexity)]
    closure: Arc<Mutex<Option<Closure<dyn FnMut(MouseEvent)>>>>,
}

impl Component for NavbarModule {
    type Message = Msg;
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            left_items: vec![
                // (BaseRoute::Dashboard, DisplayType::Icon("home", "Home")),
                // (BaseRoute::People, DisplayType::Icon("person", "Authors")),
                // (
                //     BaseRoute::Collections,
                //     DisplayType::Icon("library_books", "My Collections"),
                // ),
            ],
            right_items: vec![(
                BaseRoute::Settings,
                DisplayType::Icon("settings", "Settings"),
            )],

            search_results: None,
            closure: Arc::new(Mutex::new(None)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Close => {
                self.search_results = None;
            }

            Msg::SearchFor(value) => {
                self.search_results = None;

                ctx.link().send_future(async move {
                    Msg::SearchResults(
                        request::get_books(None, Some(0), Some(20), {
                            let mut search = FilterContainer::default();
                            search.add_query_filter(value);
                            Some(search)
                        })
                        .await,
                    )
                });
            }

            Msg::SearchResults(resp) => match resp.ok() {
                Ok(res) => self.search_results = Some(res),
                Err(err) => crate::display_error(err),
            },
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if !ctx.props().visible {
            return html! {};
        }

        let node_ref = NodeRef::default();

        html! {
            <nav class={ classes!("navbar", "navbar-expand-lg", "text-bg-dark") }>
                <div class="container-fluid">
                    // <ul class="navbar-nav">
                    //     { for self.left_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1)) }
                    // </ul>
                    <form class="d-flex" role="search">
                        <div class="input-group">
                            <input ref={ node_ref.clone() } class="form-control" type="search" placeholder="Search" aria-label="Search" />
                            <button class="btn btn-success" type="submit" onclick={
                                ctx.link().callback(move |e: MouseEvent| {
                                    e.prevent_default();

                                    let input = node_ref.cast::<HtmlInputElement>().unwrap();

                                    Msg::SearchFor(input.value())
                                })
                            }>{ "Search" }</button>
                        </div>
                    </form>

                    <ul class="navbar-nav">
                        { for self.right_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1)) }
                    </ul>
                </div>

                { self.render_dropdown_results() }
            </nav>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
        if let Some(func) = (*self.closure.lock().unwrap()).take() {
            let _ =
                body().remove_event_listener_with_callback("click", func.as_ref().unchecked_ref());
        }

        let closure = Arc::new(Mutex::default());

        let link = ctx.link().clone();
        let on_click = Closure::wrap(Box::new(move |event: MouseEvent| {
            if let Some(target) = event.target() {
                if !does_parent_contain_class(&target.unchecked_into(), "search-bar") {
                    link.send_message(Msg::Close);
                }
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        let _ = body().add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());

        *closure.lock().unwrap() = Some(on_click);

        self.closure = closure;
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        let func = (*self.closure.lock().unwrap()).take().unwrap();
        let _ = body().remove_event_listener_with_callback("click", func.as_ref().unchecked_ref());
    }
}

impl NavbarModule {
    fn render_item(route: BaseRoute, name: &DisplayType) -> Html {
        let inner = if route == BaseRoute::Settings {
            let route = if get_member_self()
                .map(|v| v.permissions.is_owner())
                .unwrap_or_default()
            {
                SettingsRoute::AdminTasks
            } else {
                SettingsRoute::General
            };

            match name {
                DisplayType::Text(v) => html! {
                    <Link<SettingsRoute> to={route} classes="nav-link px-2 text-white">{ v }</Link<SettingsRoute>>
                },
                DisplayType::Icon(icon, title) => html! {
                    <Link<SettingsRoute> to={route} classes="nav-link px-2 text-white">
                        <span class="material-icons" title={ *title }>{ icon }</span>
                    </Link<SettingsRoute>>
                },
            }
        } else {
            match name {
                DisplayType::Text(v) => html! {
                    <Link<BaseRoute> to={route} classes="nav-link px-2 text-white">{ v }</Link<BaseRoute>>
                },
                DisplayType::Icon(icon, title) => html! {
                    <Link<BaseRoute> to={route} classes="nav-link px-2 text-white">
                        <span class="material-icons" title={ *title }>{ icon }</span>
                    </Link<BaseRoute>>
                },
            }
        };

        html! {
            <li class="nav-item">{ inner }</li>
        }
    }

    fn render_dropdown_results(&self) -> Html {
        if let Some(resp) = self.search_results.as_ref() {
            html! {
                <div class="search-dropdown">
                    {
                        for resp.items.iter().map(|item| {
                            html! {
                                <BookListItemInfo
                                    small=true
                                    class="link-light"
                                    to={ BaseRoute::ViewBook { book_id: item.id } }
                                    image={ item.thumb_path.get_book_http_path().into_owned() }
                                    title={ item.title.clone() }
                                />
                            }
                        })
                    }
                </div>
            }
        } else {
            html! {}
        }
    }
}

pub enum DisplayType {
    #[allow(dead_code)]
    Text(&'static str),
    Icon(&'static str, &'static str),
}
