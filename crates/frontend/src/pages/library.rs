use books_common::{api, DisplayItem, SearchType};
use gloo_utils::document;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{HtmlInputElement, HtmlElement};
use yew::{prelude::*, html::Scope};
use yew_router::prelude::Link;

use crate::{Route, request, components::{Popup, PopupType}, util};


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
	BookSearchResults(String, api::MetadataSearchResponse),

	// Events
	OnScroll(i32),
	PosterItem(PosterItem),
	ClosePopup,

	InitEventListenerAfterMediaItems,

	Ignore
}

pub struct LibraryPage {
	on_scroll_fn: Option<Closure<dyn FnMut()>>,

	media_items: Option<Vec<DisplayItem>>,
	total_media_count: i64,

	is_fetching_media_items: bool,

	media_popup: Option<DisplayOverlay>,

	library_list_ref: NodeRef,
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
			library_list_ref: NodeRef::default()
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::ClosePopup => {
				self.media_popup = None;
			}

			Msg::InitEventListenerAfterMediaItems => {
				let lib_list_ref = self.library_list_ref.clone();
				let link = ctx.link().clone();

				let func = Closure::wrap(Box::new(move || {
					let lib_list = lib_list_ref.cast::<HtmlElement>().unwrap();

					link.send_message(Msg::OnScroll(lib_list.client_height() + lib_list.scroll_top()));
				}) as Box<dyn FnMut()>);

				let _ = self.library_list_ref.cast::<HtmlElement>().unwrap().add_event_listener_with_callback("scroll", func.as_ref().unchecked_ref());

				self.on_scroll_fn = Some(func);
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

			Msg::BookSearchResults(search_value, resp) => {
				if let Some(DisplayOverlay::SearchForBook { response, input_value, .. }) = self.media_popup.as_mut() {
					*response = Some(resp);
					*input_value = Some(search_value);
				}
			}

			Msg::OnScroll(scroll_y) => {
				let scroll_height = self.library_list_ref.cast::<HtmlElement>().unwrap().scroll_height();

				if scroll_height - scroll_y < 600 && self.can_req_more() {
					ctx.link().send_message(Msg::RequestMediaItems);
				}
			}

			Msg::PosterItem(item) => match item {
				PosterItem::ShowPopup(new_disp) => {
					if let Some(old_disp) = self.media_popup.as_mut() {
						if *old_disp == new_disp {
							self.media_popup = None;
						} else {
							self.media_popup = Some(new_disp);
						}
					} else {
						self.media_popup = Some(new_disp);
					}
				}

				PosterItem::UpdateMeta(meta_id) => {
					ctx.link()
					.send_future(async move {
						request::update_metadata(&api::PostMetadataBody::AutoMatchByMetaId(meta_id)).await;

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
				<div class="main-content-view">
					<div class="library-list normal" ref={ self.library_list_ref.clone() }>
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
														Self::on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::SearchForBook { meta_id, input_value: None, response: None })))
													}>{ "Search For Book" }</div>
													<div class="menu-item" yew-close-popup="">{ "Delete" }</div>
													<div class="menu-item" yew-close-popup="" onclick={
														Self::on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::Info { meta_id })))
													}>{ "Show Info" }</div>
												</div>
											</Popup>
										}
									}

									&DisplayOverlay::SearchForBook { meta_id, ref input_value, ref response } => {
										let input_id = "external-book-search-input";

										let input_value = if let Some(v) = input_value {
											v.to_string()
										} else {
											let items = self.media_items.as_ref().unwrap();
											items.iter().find(|v| v.id == meta_id).unwrap().title.clone()
										};

										html! {
											<Popup
												type_of={ PopupType::FullOverlay }
												on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
												classes={ classes!("external-book-search-popup") }
											>
												<h1>{"Book Search"}</h1>

												<form>
													<input id={input_id} name="book_search" placeholder="Search For Title" value={ input_value } />
													<button onclick={
														ctx.link().callback_future(move |e: MouseEvent| async move {
															e.prevent_default();

															let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

															Msg::BookSearchResults(input.value(), request::search_for(&input.value(), SearchType::Book).await)
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
																									let item = item.as_book();

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
																												<p>{ item.description.clone().map(|mut v| { util::truncate_on_indices(&mut v, 300); v }).unwrap_or_default() }</p>
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
				</div>
			}
		} else {
			html! {
				<h1>{ "Loading..." }</h1>
			}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		if self.on_scroll_fn.is_none() && self.library_list_ref.get().is_some() {
			ctx.link().send_message(Msg::InitEventListenerAfterMediaItems);
		} else if first_render {
			ctx.link().send_message(Msg::RequestMediaItems);
		}
	}

	fn destroy(&mut self, _ctx: &Context<Self>) {
		// TODO: Determine if still needed.
		if let Some(f) = self.on_scroll_fn.take() {
			let _ = self.library_list_ref.cast::<HtmlElement>().unwrap().remove_event_listener_with_callback("scroll", f.as_ref().unchecked_ref());
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
			<Link<Route> to={Route::ViewMeta { meta_id: item.id as usize }} classes={ classes!("library-item") }>
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
		input_value: Option<String>,
		response: Option<api::MetadataSearchResponse>
	},
}

impl PartialEq for DisplayOverlay {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Info { meta_id: l_id }, Self::Info { meta_id: r_id }) => l_id == r_id,
			(Self::More { meta_id: l_id, .. }, Self::More { meta_id: r_id, .. }) => l_id == r_id,
			(
				Self::SearchForBook { meta_id: l_id, input_value: l_val, .. },
				Self::SearchForBook { meta_id: r_id, input_value: r_val, .. }
			) => l_id == r_id && l_val == r_val,

			_ => false
		}
	}
}