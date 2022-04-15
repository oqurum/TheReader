use books_common::{api::{MetadataSearchResponse, PostMetadataBody}, SearchType};
use gloo_utils::document;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{request, util};

use super::{Popup, PopupType};


#[derive(Properties, PartialEq)]
pub struct Property {
	#[prop_or_default]
    pub classes: Classes,

	pub on_close: Callback<()>,

	pub meta_id: usize,
	pub input_value: String,
}


pub enum Msg {
	BookSearchResponse(String, MetadataSearchResponse),

	Ignore,
}


pub struct PopupSearchBook {
	cached_posters: Option<MetadataSearchResponse>,
	input_value: String,
}

impl Component for PopupSearchBook {
	type Message = Msg;
	type Properties = Property;

	fn create(ctx: &Context<Self>) -> Self {
		Self {
			cached_posters: None,
			input_value: ctx.props().input_value.clone(),
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Ignore => {
				return false;
			}

			Msg::BookSearchResponse(search, resp) => {
				self.cached_posters = Some(resp);
				self.input_value = search;
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		let input_id = "external-book-search-input";

		let meta_id = ctx.props().meta_id;

		html! {
			<Popup
				type_of={ PopupType::FullOverlay }
				on_close={ ctx.props().on_close.clone() }
				classes={ classes!("external-book-search-popup") }
			>
				<h1>{"Book Search"}</h1>

				<form>
					<input id={input_id} name="book_search" placeholder="Search For Title" value={ self.input_value.clone() } />
					<button onclick={
						ctx.link().callback_future(move |e: MouseEvent| async move {
							e.prevent_default();

							let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

							Msg::BookSearchResponse(input.value(), request::search_for(&input.value(), SearchType::Book).await)
						})
					}>{ "Search" }</button>
				</form>

				<div class="external-book-search-container">
					{
						if let Some(resp) = self.cached_posters.as_ref() {
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
																						request::update_metadata(
																							meta_id,
																							&PostMetadataBody::UpdateMetaBySource(source)
																						).await;

																						Msg::Ignore
																					}
																				})
																			}
																		>
																			<img src={ item.thumbnail.to_string() } />
																			<div class="book-info">
																				<h4>{ item.name.clone() }</h4>
																				<span>{ item.author.clone().unwrap_or_default() }</span>
																				<p>{ item.description.clone()
																						.map(|mut v| { util::truncate_on_indices(&mut v, 300); v })
																						.unwrap_or_default() }
																				</p>
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