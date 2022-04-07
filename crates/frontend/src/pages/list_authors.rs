use books_common::{api, Person, SearchType};
use gloo_utils::document;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{HtmlElement, HtmlInputElement};
use yew::{prelude::*, html::Scope};

use crate::{request, components::{PopupType, Popup}, util};


#[derive(Properties, PartialEq)]
pub struct Property {
}

#[derive(Clone)]
pub enum Msg {
	// Requests
	RequestPeople,

	// Results
	PeopleListResults(api::GetPeopleResponse),
	PersonSearchResults(String, api::MetadataSearchResponse),

	// Events
	OnScroll(i32),
	PosterItem(PosterItem),
	ClosePopup,

	InitEventListenerAfterMediaItems,

	Ignore
}

pub struct AuthorListPage {
	on_scroll_fn: Option<Closure<dyn FnMut()>>,

	media_items: Option<Vec<Person>>,
	total_media_count: usize,

	is_fetching_authors: bool,

	media_popup: Option<DisplayOverlay>,

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
			media_popup: None,
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

			Msg::PersonSearchResults(search_value, resp) => {
				if let Some(DisplayOverlay::SearchForPerson { response, input_value, .. }) = self.media_popup.as_mut() {
					*response = Some(resp);
					*input_value = Some(search_value);
				}
			}

			Msg::OnScroll(scroll_y) => {
				let scroll_height = self.author_list_ref.cast::<HtmlElement>().unwrap().scroll_height();

				if scroll_height - scroll_y < 600 && self.can_req_more() {
					ctx.link().send_message(Msg::RequestPeople);
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

				// PosterItem::UpdateMeta(file_id) => {
				// 	ctx.link()
				// 	.send_future(async move {
				// 		request::update_metadata(&api::PostMetadataBody::AutoMatchByFileId(file_id)).await;

				// 		Msg::Ignore
				// 	});
				// }
			}

			Msg::ClosePopup => {
				self.media_popup = None;
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

						{
							if let Some(overlay_type) = self.media_popup.as_ref() {
								match overlay_type {
									DisplayOverlay::Info { person_id: _ } => {
										html! {
											<Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
												<h1>{"Info"}</h1>
											</Popup>
										}
									}

									&DisplayOverlay::More { person_id, mouse_pos } => {
										html! {
											<Popup type_of={ PopupType::AtPoint(mouse_pos.0, mouse_pos.1) } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
												<div class="menu-list">
													<div class="menu-item" yew-close-popup="">{ "Start Reading" }</div>
													// <div class="menu-item" yew-close-popup="" onclick={
													// 	Self::on_click_prevdef(ctx.link(), Msg::PosterItem(PosterItem::UpdateMeta(person_id)))
													// }>{ "Refresh Metadata" }</div>
													<div class="menu-item" yew-close-popup="" onclick={
														Self::on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::SearchForPerson { person_id, input_value: None, response: None })))
													}>{ "Search For Person" }</div>
													<div class="menu-item" yew-close-popup="">{ "Delete" }</div>
													<div class="menu-item" yew-close-popup="" onclick={
														Self::on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::Info { person_id })))
													}>{ "Show Info" }</div>
												</div>
											</Popup>
										}
									}

									&DisplayOverlay::SearchForPerson { person_id, ref input_value, ref response } => {
										let input_id = "external-person-search-input";

										let input_value = if let Some(v) = input_value {
											v.to_string()
										} else {
											let items = self.media_items.as_ref().unwrap();
											items.iter().find(|v| v.id == person_id).unwrap().name.clone()
										};

										html! {
											<Popup
												type_of={ PopupType::FullOverlay }
												on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
												classes={ classes!("external-person-search-popup") }
											>
												<h1>{"Person Search"}</h1>

												<form>
													<input id={input_id} name="person_search" placeholder="Search For Person" value={ input_value } />
													<button onclick={
														ctx.link().callback_future(move |e: MouseEvent| async move {
															e.prevent_default();

															let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

															Msg::PersonSearchResults(input.value(), request::search_for(&input.value(), SearchType::Person).await)
														})
													}>{ "Search" }</button>
												</form>

												<div class="external-person-search-container">
													{
														if let Some(resp) = response {
															html! {
																{
																	for resp.items.iter()
																		.map(|(site, items)| {
																			html! {
																				<>
																					<h2>{ site.clone() }</h2>
																					<div class="person-search-items">
																						{
																							for items.iter()
																								.map(|item| {
																									let item = item.as_person();

																									let source = item.source.clone();

																									html! { // TODO: Place into own component.
																										<div
																											class="person-search-item"
																											yew-close-popup=""
																											onclick={
																												ctx.link()
																												.callback_future(move |_| {
																													let source = source.clone();

																													async move {
																														// request::update_metadata(&api::PostMetadataBody::UpdateMetaBySource {
																														// 	person_id,
																														// 	source
																														// }).await;

																														Msg::Ignore
																													}
																												})
																											}
																										>
																											<img src={ item.cover_image.clone().unwrap_or_default() } />
																											<div class="person-info">
																												<h4>{ item.name.clone() }</h4>
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
	fn render_media_item(item: &Person, scope: &Scope<Self>) -> Html {
		let person_id = item.id;
		let on_click_more = scope.callback(move |e: MouseEvent| {
			e.prevent_default();
			e.stop_propagation();

			Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::More { person_id, mouse_pos: (e.page_x(), e.page_y()) }))
		});

		html! {
			<div class="person-container">
				<div class="photo">
					<div class="bottom-right">
						<span class="material-icons" onclick={on_click_more} title="More Options">{ "more_horiz" }</span>
					</div>
					<img src="/images/missingperson.jpg" />
				</div>
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


#[derive(Clone)]
pub enum PosterItem {
	// Poster Specific Buttons
	ShowPopup(DisplayOverlay),

	// Popup Events
	// UpdateMeta(i64),
}


#[derive(Clone)]
pub enum DisplayOverlay {
	Info {
		person_id: i64
	},

	More {
		person_id: i64,
		mouse_pos: (i32, i32)
	},

	SearchForPerson {
		person_id: i64,
		input_value: Option<String>,
		response: Option<api::MetadataSearchResponse>
	},
}

impl PartialEq for DisplayOverlay {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Info { person_id: l_id }, Self::Info { person_id: r_id }) => l_id == r_id,
			(Self::More { person_id: l_id, .. }, Self::More { person_id: r_id, .. }) => l_id == r_id,
			(
				Self::SearchForPerson { person_id: l_id, input_value: l_val, .. },
				Self::SearchForPerson { person_id: r_id, input_value: r_val, .. }
			) => l_id == r_id && l_val == r_val,

			_ => false
		}
	}
}