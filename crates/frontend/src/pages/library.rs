use books_common::{api, DisplayItem};
use gloo_utils::{document, window};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::HtmlInputElement;
use yew::{prelude::*, html::Scope};
use yew_router::prelude::Link;

use crate::{Route, request, components::{Popup, PopupType}};


#[derive(Properties, PartialEq)]
pub struct Property {
	pub library_id: i64,
}

#[derive(Clone)]
pub enum Msg {
	// Requests
	RequestMediaItems,

	// Results
	MediaListResults(api::GetBookListResponse),
	BookSearchResults(api::MetadataSearchResponse),

	// Events
	OnScroll(i32),
	PosterItem(PosterItem),
	ClosePopup,

	Ignore
}

pub struct LibraryPage {
	on_scroll_fn: Option<Closure<dyn FnMut()>>,

	media_items: Option<Vec<DisplayItem>>,
	total_media_count: i64,

	is_fetching_media_items: bool,

	media_popup: Option<DisplayOverlay>,
}

impl Component for LibraryPage {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			on_scroll_fn: None,
			media_items: None,
			total_media_count: 0,
			is_fetching_media_items: false,
			media_popup: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::ClosePopup => {
				self.media_popup = None;
			}

			Msg::RequestMediaItems => {
				if self.is_fetching_media_items {
					return false;
				}

				self.is_fetching_media_items = true;

				let offset = Some(self.media_items.as_ref().map(|v| v.len()).unwrap_or_default()).filter(|v| *v != 0);

				let library = ctx.props().library_id;

				ctx.link()
				.send_future(async move {
					Msg::MediaListResults(request::get_books(library, offset, None).await)
				});
			}

			Msg::MediaListResults(mut resp) => {
				self.is_fetching_media_items = false;
				self.total_media_count = resp.count;

				if let Some(items) = self.media_items.as_mut() {
					items.append(&mut resp.items);
				} else {
					self.media_items = Some(resp.items);
				}
			}

			Msg::BookSearchResults(resp) => {
				if let Some(DisplayOverlay::SearchForBook { response, .. }) = self.media_popup.as_mut() {
					*response = Some(resp);
				}
			}

			Msg::OnScroll(scroll_y) => {
				let scroll_height = document().body().unwrap().scroll_height();

				if scroll_height - scroll_y < 600 && self.can_req_more() {
					ctx.link().send_message(Msg::RequestMediaItems);
				}
			}

			Msg::PosterItem(item) => match item {
				PosterItem::ShowPopup(type_of) => {
					self.media_popup = Some(type_of);
				}

				PosterItem::UpdateMeta(file_id) => {
					ctx.link()
					.send_future(async move {
						request::update_metadata(&api::PostMetadataBody::AutoMatchByFileId(file_id)).await;

						Msg::Ignore
					});
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
				<div class="library-list normal">
					{ for items.iter().map(|item| Self::render_media_item(item, ctx.link())) }
					// { for (0..remaining).map(|_| Self::render_placeholder_item()) }

					{
						if let Some(overlay_type) = self.media_popup.as_ref() {
							match overlay_type {
								DisplayOverlay::Info { meta_id: _ } => {
									html! {
										<Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
											<h1>{"Info"}</h1>
										</Popup>
									}
								}

								&DisplayOverlay::More { meta_id, mouse_pos } => {
									html! {
										<Popup type_of={ PopupType::AtPoint(mouse_pos.0, mouse_pos.1) } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
											<div class="menu-list">
												<div class="menu-item" yew-close-popup="">{ "Start Reading" }</div>
												<div class="menu-item" yew-close-popup="" onclick={
													Self::on_click_prevdef(ctx.link(), Msg::PosterItem(PosterItem::UpdateMeta(meta_id)))
												}>{ "Refresh Metadata" }</div>
												<div class="menu-item" yew-close-popup="" onclick={
													Self::on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::SearchForBook { meta_id, title: None, response: None })))
												}>{ "Search For Book" }</div>
												<div class="menu-item" yew-close-popup="">{ "Delete" }</div>
												<div class="menu-item" yew-close-popup="" onclick={
													Self::on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::Info { meta_id })))
												}>{ "Show Info" }</div>
											</div>
										</Popup>
									}
								}

								&DisplayOverlay::SearchForBook { meta_id, ref title, ref response } => {
									let input_id = "external-book-search-input";

									html! {
										<Popup
											type_of={ PopupType::FullOverlay }
											on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
											classes={ classes!("external-book-search-popup") }
										>
											<h1>{"Book Search"}</h1>

											<form>
												<input id={input_id} name="book_search" placeholder="Search For Title" value={ title.clone().unwrap_or_default() } />
												<button onclick={
													ctx.link().callback_future(move |e: MouseEvent| async move {
														e.prevent_default();

														let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

														Msg::BookSearchResults(request::search_for_books(&input.value()).await)
													})
												}>{ "Search" }</button>
											</form>

											<div class="external-book-search-container">
												{
													if let Some(resp) = response {
														html! {
															{
																for resp.items.iter()
																	.map(|(site, items)| {
																		html! {
																			<>
																				<h2>{ site.clone() }</h2>
																				<div class="book-search-items">
																					{
																						for items.iter()
																							.map(|item| {
																								let source = item.source.clone();

																								html! { // TODO: Place into own component.
																									<div
																										class="book-search-item"
																										yew-close-popup=""
																										onclick={
																											ctx.link()
																											.callback_future(move |_| {
																												let source = source.clone();

																												async move {
																													request::update_metadata(&api::PostMetadataBody::UpdateMetaBySource {
																														meta_id,
																														source
																													}).await;

																													Msg::Ignore
																												}
																											})
																										}
																									>
																										<img src={ item.thumbnail.clone().unwrap_or_default() } />
																										<div class="book-info">
																											<h4>{ item.name.clone() }</h4>
																											<span>{ item.author.clone().unwrap_or_default() }</span>
																											<p>{ item.description.clone().map(|mut v| { v.truncate(300); v }).unwrap_or_default() }</p>
																										</div>
																									</div>
																								}
																							})
																					}
																				</div>
																			</>
																		}
																	})
															}
														}
													} else {
														html! {}
													}
												}
											</div>
										</Popup>
									}
								}
							}
						} else {
							html! {}
						}
					}
				</div>
			}
		} else {
			html! {
				<h1>{ "Loading..." }</h1>
			}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		if first_render {
			let link = ctx.link().clone();
			let func = Closure::wrap(Box::new(move || {
				link.send_message(Msg::OnScroll(
					(
						window().inner_height().map(|v| v.as_f64().unwrap()).unwrap_or_default() +
						window().scroll_y().unwrap_or_default()
					) as i32
				));
			}) as Box<dyn FnMut()>);

			let _ = document().add_event_listener_with_callback("scroll", func.as_ref().unchecked_ref());

			self.on_scroll_fn = Some(func);

			ctx.link().send_message(Msg::RequestMediaItems);
		}
	}

	fn destroy(&mut self, _ctx: &Context<Self>) {
		if let Some(f) = self.on_scroll_fn.take() {
			let _ = document().remove_event_listener_with_callback("scroll", f.as_ref().unchecked_ref());
		}
	}
}

impl LibraryPage {
	/// A Callback which calls "prevent_default" and "stop_propagation"
	fn on_click_prevdef_stopprop(scope: &Scope<Self>, msg: Msg) -> Callback<MouseEvent> {
		scope.callback(move |e: MouseEvent| {
			e.prevent_default();
			e.stop_propagation();
			msg.clone()
		})
	}

	/// A Callback which calls "prevent_default"
	fn on_click_prevdef(scope: &Scope<Self>, msg: Msg) -> Callback<MouseEvent> {
		scope.callback(move |e: MouseEvent| {
			e.prevent_default();
			msg.clone()
		})
	}

	// TODO: Move into own struct.
	fn render_media_item(item: &DisplayItem, scope: &Scope<Self>) -> Html {
		let meta_id = item.id;
		let on_click_more = scope.callback(move |e: MouseEvent| {
			e.prevent_default();
			e.stop_propagation();

			Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::More { meta_id, mouse_pos: (e.page_x(), e.page_y()) }))
		});

		html! {
			<Link<Route> to={Route::ReadBook { book_id: item.id as usize }} classes={ classes!("library-item") }>
				<div class="poster">
					<div class="bottom-right">
						<span class="material-icons" onclick={on_click_more} title="More Options">{ "more_horiz" }</span>
					</div>
					<img src={ if item.has_thumbnail { format!("/api/metadata/{}/thumbnail", item.id) } else { String::from("/images/missingthumbnail.jpg") } } />
				</div>
				<div class="info">
					<div class="title" title={ item.title.clone() }>{ item.title.clone() }</div>
					{
						if let Some(author) = item.cached.author.as_ref() {
							html! {
								<div class="author" title={ author.clone() }>{ author.clone() }</div>
							}
						} else {
							html! {}
						}
					}
				</div>
			</Link<Route>>
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

#[derive(Clone)]
pub enum PosterItem {
	// Poster Specific Buttons
	ShowPopup(DisplayOverlay),

	// Popup Events
	UpdateMeta(i64),
}

#[derive(Clone)]
pub enum DisplayOverlay {
	Info {
		meta_id: i64
	},

	More {
		meta_id: i64,
		mouse_pos: (i32, i32)
	},

	SearchForBook {
		meta_id: i64,
		title: Option<String>,
		response: Option<api::MetadataSearchResponse>
	},
}