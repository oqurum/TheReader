use books_common::{api, Person};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::HtmlElement;
use yew::{prelude::*, html::Scope};

use crate::request;


#[derive(Properties, PartialEq)]
pub struct Property {
}

#[derive(Clone)]
pub enum Msg {
	// Requests
	RequestPeople,

	// Results
	PeopleListResults(api::GetPeopleResponse),

	// Events
	OnScroll(i32),

	InitEventListenerAfterMediaItems,

	Ignore
}

pub struct AuthorListPage {
	on_scroll_fn: Option<Closure<dyn FnMut()>>,

	media_items: Option<Vec<Person>>,
	total_media_count: usize,

	is_fetching_authors: bool,

	author_list_ref: NodeRef,
}

impl Component for AuthorListPage {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			on_scroll_fn: None,
			media_items: None,
			total_media_count: 0,
			is_fetching_authors: false,
			author_list_ref: NodeRef::default()
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::InitEventListenerAfterMediaItems => {
				let lib_list_ref = self.author_list_ref.clone();
				let link = ctx.link().clone();

				let func = Closure::wrap(Box::new(move || {
					let lib_list = lib_list_ref.cast::<HtmlElement>().unwrap();

					link.send_message(Msg::OnScroll(lib_list.client_height() + lib_list.scroll_top()));
				}) as Box<dyn FnMut()>);

				let _ = self.author_list_ref.cast::<HtmlElement>().unwrap().add_event_listener_with_callback("scroll", func.as_ref().unchecked_ref());

				self.on_scroll_fn = Some(func);
			}

			Msg::RequestPeople => {
				if self.is_fetching_authors {
					return false;
				}

				self.is_fetching_authors = true;

				let offset = Some(self.media_items.as_ref().map(|v| v.len()).unwrap_or_default()).filter(|v| *v != 0);

				ctx.link()
				.send_future(async move {
					Msg::PeopleListResults(request::get_people(offset, None).await)
				});
			}

			Msg::PeopleListResults(mut resp) => {
				self.is_fetching_authors = false;
				self.total_media_count = resp.total;

				if let Some(items) = self.media_items.as_mut() {
					items.append(&mut resp.items);
				} else {
					self.media_items = Some(resp.items);
				}
			}

			Msg::OnScroll(scroll_y) => {
				let scroll_height = self.author_list_ref.cast::<HtmlElement>().unwrap().scroll_height();

				if scroll_height - scroll_y < 600 && self.can_req_more() {
					ctx.link().send_message(Msg::RequestPeople);
				}
			}

			Msg::Ignore => return false,
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(items) = self.media_items.as_deref() {
			// TODO: Placeholders
			// let remaining = (self.total_media_count as usize - items.len()).min(50);

			html! {
				<div class="person-view-container">
					<div class="person-list" ref={ self.author_list_ref.clone() }>
						{ for items.iter().map(|item| Self::render_media_item(item, ctx.link())) }
						// { for (0..remaining).map(|_| Self::render_placeholder_item()) }
					</div>
				</div>
			}
		} else {
			html! {
				<h1>{ "Loading..." }</h1>
			}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		if self.on_scroll_fn.is_none() && self.author_list_ref.get().is_some() {
			ctx.link().send_message(Msg::InitEventListenerAfterMediaItems);
		} else if first_render {
			ctx.link().send_message(Msg::RequestPeople);
		}
	}

	fn destroy(&mut self, _ctx: &Context<Self>) {
		// TODO: Determine if still needed.
		if let Some(f) = self.on_scroll_fn.take() {
			let _ = self.author_list_ref.cast::<HtmlElement>().unwrap().remove_event_listener_with_callback("scroll", f.as_ref().unchecked_ref());
		}
	}
}

impl AuthorListPage {
	// TODO: Move into own struct.
	fn render_media_item(item: &Person, scope: &Scope<Self>) -> Html {
		html! {
			<div class="person-container">
				<div class="photo"><img src="/images/missingperson.jpg" /></div>
				<span class="title">{ item.name.clone() }</span>
			</div>
		}
	}

	// fn render_placeholder_item() -> Html {
	// 	html! {
	// 		<div class="library-item placeholder">
	// 			<div class="poster"></div>
	// 			<div class="info">
	// 				<a class="author"></a>
	// 				<a class="title"></a>
	// 			</div>
	// 		</div>
	// 	}
	// }

	pub fn can_req_more(&self) -> bool {
		let count = self.media_items.as_ref().map(|v| v.len()).unwrap_or_default();

		count != 0 && count != self.total_media_count as usize
	}
}