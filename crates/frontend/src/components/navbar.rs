use std::sync::{Mutex, Arc};

use books_common::api::{GetBookListResponse, self};
use gloo_utils::{document, body};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::components::Link;

use crate::{Route, request, util};

pub enum Msg {
	Close,
	SearchFor(String),
	SearchResults(GetBookListResponse),
}

pub struct NavbarModule {
	left_items: Vec<(Route, DisplayType)>,
	right_items: Vec<(Route, DisplayType)>,

	search_results: Option<GetBookListResponse>,
	#[allow(clippy::type_complexity)]
	closure: Arc<Mutex<Option<Closure<dyn FnMut(MouseEvent)>>>>,
}

impl Component for NavbarModule {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			left_items: vec![
				(Route::Dashboard, DisplayType::Icon("home", "Home")),
			],
			right_items: vec![
				(Route::Options, DisplayType::Icon("settings", "Settings")),
			],

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

				// TODO: Auto Close. Temp
				if value.is_empty() {
					return true;
				}

				ctx.link().send_future(async move {
					Msg::SearchResults(request::get_books(
						None,
						Some(0),
						Some(20),
						Some(api::SearchQuery {
							query: Some(value),
							source: None,
						})
					).await)
				});
			}

			Msg::SearchResults(res) => self.search_results = Some(res),
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		let input_id = "book-search-input";

		html! {
			<div class="navbar-module">
				<div class="left-content">
				{
					for self.left_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1))
				}
				</div>
				<div class="center-content">
					<div class="search-bar">
						<input id={input_id} placeholder="Search" class="alternate" />
						<button class="alternate" onclick={
							ctx.link().callback(move |e: MouseEvent| {
								e.prevent_default();

								let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

								Msg::SearchFor(input.value())
							})
						}>{ "Search" }</button>
					</div>

					{ self.render_dropdown_results() }
				</div>
				<div class="right-content">
				{
					for self.right_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1))
				}
				</div>
			</div>
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
		if let Some(func) = (*self.closure.lock().unwrap()).take() {
			let _ = body().remove_event_listener_with_callback("click", func.as_ref().unchecked_ref());
		}

		let closure = Arc::new(Mutex::default());

		let link = ctx.link().clone();
		let on_click = Closure::wrap(Box::new(move |event: MouseEvent| {
			if let Some(target) = event.target() {
				if !util::does_parent_contain_class(&target.unchecked_into(), "search-bar") {
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
	fn render_item(route: Route, name: &DisplayType) -> Html {
		match name {
			DisplayType::Text(v) => html! {
				<Link<Route> to={route}>{ v }</Link<Route>>
			},
			DisplayType::Icon(icon, title) => html! {
				<Link<Route> to={route}>
					<span class="material-icons" title={ *title }>{ icon }</span>
				</Link<Route>>
			}
		}
	}

	fn render_dropdown_results(&self) -> Html {
		if let Some(resp) = self.search_results.as_ref() {
			html! {
				<div class="search-dropdown">
					{
						for resp.items.iter().map(|item| {
							html_nested! {
								<Link<Route> to={Route::ViewMeta { meta_id: item.id }} classes={ classes!("search-item") }>
									<div class="poster max-vertical">
										<img src={ item.get_thumb_url() } />
									</div>
									<div class="info">
										<h5 class="book-name">{ item.title.clone() }</h5>
									</div>
								</Link<Route>>
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
	Text(&'static str),
	Icon(&'static str, &'static str),
}