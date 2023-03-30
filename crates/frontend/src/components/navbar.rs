use std::{sync::{Arc, Mutex}, rc::Rc};

use common::{api::WrappingResponse, util::does_parent_contain_class};
use common_local::{api::GetBookListResponse, filter::FilterContainer, ThumbnailStoreExt};
use gloo_utils::body;
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::{components::Link, Routable};

use crate::{
    components::BookListItemInfo, pages::settings::SettingsRoute, request,
    BaseRoute, AppState,
};

#[derive(PartialEq, Eq, Properties)]
pub struct Property {
    pub visible: bool,
}

pub enum Msg {
    Close,

    SearchInput,
    SearchResults(usize, WrappingResponse<GetBookListResponse>),

    ContextChanged(Rc<AppState>),
}

pub struct NavbarModule {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    // left_items: Vec<(BaseRoute, DisplayType)>,
    right_items: Vec<(BaseRoute, DisplayType)>,

    input_ref: NodeRef,

    search_results: Vec<Option<GetBookListResponse>>,
    #[allow(clippy::type_complexity)]
    closure: Arc<Mutex<Option<Closure<dyn FnMut(MouseEvent)>>>>,
}

impl Component for NavbarModule {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");

        Self {
            state,
            _listener,

            // left_items: vec![
            //     // (BaseRoute::Dashboard, DisplayType::Icon("home", "Home")),
            //     // (BaseRoute::People, DisplayType::Icon("person", "Authors")),
            //     // (
            //     //     BaseRoute::Collections,
            //     //     DisplayType::Icon("library_books", "My Collections"),
            //     // ),
            // ],
            right_items: vec![(
                BaseRoute::Settings,
                DisplayType::Icon("settings", "Settings"),
            )],

            input_ref: NodeRef::default(),
            search_results: Vec::new(),
            closure: Arc::new(Mutex::new(None)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => self.state = state,

            Msg::Close => {
                self.search_results.clear();
            }

            Msg::SearchInput => {
                // TODO: Remove if and properly handle.
                if self.search_results.iter().all(|v| v.is_some()) {
                    self.search_results.clear();

                    let limit = if self.state.libraries.len() > 1 { 5 } else { 10 };

                    self.search_results = vec![Option::None; self.state.libraries.len()];

                    let mut search = FilterContainer::default();
                    search.add_query_filter(self.input_ref.cast::<HtmlInputElement>().unwrap().value());

                    for (index, coll) in self.state.libraries.iter().enumerate() {
                        let search = search.clone();
                        let library_id = coll.id;

                        ctx.link().send_future(async move {
                            Msg::SearchResults(
                                index,
                                request::get_books(
                                    Some(library_id),
                                    Some(0),
                                    Some(limit),
                                    Some(search)
                                )
                                .await,
                            )
                        });
                    }
                }
            }

            Msg::SearchResults(index, resp) => match resp.ok() {
                Ok(res) => self.search_results[index] = Some(res),
                Err(err) => crate::display_error(err),
            },
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if !ctx.props().visible {
            return html! {};
        }

        html! {
            <nav class={ classes!("navbar", "navbar-expand-sm", "text-bg-dark") }>
                <div class="container-fluid">
                    <button
                        class="navbar-toggler navbar-dark"
                        type="button"
                        data-bs-toggle="collapse"
                        data-bs-target="#sidebarToggle"
                        aria-controls="sidebarToggle"
                        aria-expanded="false"
                        aria-label="Toggle sidebar"
                    >
                        <span class="navbar-toggler-icon"></span>
                    </button>

                    // <ul class="navbar-nav">
                    //     { for self.left_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1)) }
                    // </ul>
                    <form class="d-flex col-8 col-sm-auto" role="search">
                        <div class="input-group">
                            <input ref={ self.input_ref.clone() } class="form-control" type="search" placeholder="Search" aria-label="Search" />
                            <button class="btn btn-success" type="submit" onclick={
                                ctx.link().callback(move |e: MouseEvent| {
                                    e.prevent_default();

                                    Msg::SearchInput
                                })
                            }>{ "Search" }</button>
                        </div>
                    </form>

                    <ul class="navbar-nav">
                        { for self.right_items.iter().map(|item| self.render_item(item.0.clone(), &item.1)) }
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
    fn render_item(&self, route: BaseRoute, name: &DisplayType) -> Html {
        let inner = if route == BaseRoute::Settings {
            let route = if self.state.member.as_ref()
                .map(|v| v.permissions.is_owner())
                .unwrap_or_default()
            {
                SettingsRoute::AdminTasks
            } else {
                SettingsRoute::MemberGeneral
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
        if self.search_results.is_empty() {
            return html! {};
        }

        let mut search = FilterContainer::default();
        search.add_query_filter(self.input_ref.cast::<HtmlInputElement>().unwrap().value());
        let query = serde_qs::to_string(&search).unwrap_throw();

        html! {
            <div class="search-dropdown">
            {
                for self.state.libraries.iter().zip(self.search_results.iter()).map(|(lib, results)| {
                    let url = BaseRoute::ViewLibrary { id: lib.id }.to_path();

                    html! {
                            <>
                                <div class="d-grid p-2">
                                    // TODO: Use <Link />
                                    <a
                                        href={ format!("{url}?{query}") }
                                        type="button"
                                        class="btn btn-secondary"
                                    >{ format!("View '{}' Library", lib.name) }</a>
                                </div>

                                {
                                    if let Some(res) = results {
                                        html! {
                                            for res.items.iter().map(|item| html! {
                                                <BookListItemInfo
                                                    small=true
                                                    class="link-light"
                                                    to={ BaseRoute::ViewBook { book_id: item.id } }
                                                    image={ item.thumb_path.get_book_http_path().into_owned() }
                                                    title={ item.title.clone() }
                                                />
                                            })
                                        }
                                    } else {
                                        html! {
                                            <h6>{ "Loading..." }</h6>
                                        }
                                    }
                                }
                            </>
                        }
                    })
                }
            </div>
        }
    }
}

pub enum DisplayType {
    #[allow(dead_code)]
    Text(&'static str),
    Icon(&'static str, &'static str),
}
