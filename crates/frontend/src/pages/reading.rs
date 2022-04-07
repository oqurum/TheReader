// TODO: Handle resizing.
// TODO: Allow custom sizes.

use std::{rc::Rc, sync::Mutex};

use books_common::{MediaItem, api::{GetBookIdResponse, GetChaptersResponse}, Progression};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, Element};
use yew::prelude::*;

use crate::{request, components::reader::{LoadedChapters, ChapterDisplay}};
use crate::components::reader::Reader;
use crate::components::notes::Notes;


#[derive(Clone, Copy, PartialEq)]
pub enum SidebarType {
	Notes,
	Settings
}

pub enum Msg {
	// Event
	ToggleSidebar(SidebarType),
	OnChangeSelection(ChapterDisplay),
	UpdateDimensions,

	// Send
	SendGetChapters(usize, usize),

	// Retrive
	RetrieveBook(GetBookIdResponse),
	RetrievePages(GetChaptersResponse),
}

#[derive(Properties, PartialEq)]
pub struct Property {
	pub id: usize
}

pub struct ReadingBook {
	book_display: ChapterDisplay,
	progress: Rc<Mutex<Option<Progression>>>,
	book: Option<Rc<MediaItem>>,
	chapters: Rc<Mutex<LoadedChapters>>,
	last_grabbed_count: usize,
	// TODO: Cache pages

	book_dimensions: (Option<i32>, Option<i32>),

	sidebar_visible: Option<SidebarType>,

	// Refs
	ref_width_input: NodeRef,
	ref_height_input: NodeRef,
	ref_book_container: NodeRef,
}

impl Component for ReadingBook {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			book_display: ChapterDisplay::DoublePage,
			chapters: Rc::new(Mutex::new(LoadedChapters::new())),
			last_grabbed_count: 0,
			progress: Rc::new(Mutex::new(None)),
			book: None,

			book_dimensions: (Some(1040), Some(548)),

			sidebar_visible: None,

			ref_width_input: NodeRef::default(),
			ref_height_input: NodeRef::default(),
			ref_book_container: NodeRef::default(),
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::UpdateDimensions => {
				let width = self.ref_width_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;
				let height = self.ref_height_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;
				self.book_dimensions = (Some(width).filter(|v| *v > 0), Some(height).filter(|v| *v > 0));
			}

			Msg::OnChangeSelection(change) => {
				if let Some(dim_x) = self.book_dimensions.0.as_mut() {
					if self.book_display != change {
						match change {
							ChapterDisplay::SinglePage => *dim_x /= 2,
							ChapterDisplay::DoublePage => *dim_x *= 2,
						}
					}
				}

				self.book_display = change;
			}

			Msg::ToggleSidebar(type_of) => {
				match self.sidebar_visible {
					Some(v) if v == type_of => { self.sidebar_visible = None; },
					_ => self.sidebar_visible = Some(type_of),
				}
			}

			Msg::RetrievePages(mut info) => {
				let mut chap_container = self.chapters.lock().unwrap();

				self.last_grabbed_count = info.limit;
				chap_container.total = info.total;

				chap_container.chapters.append(&mut info.items);
			}

			Msg::RetrieveBook(resp) => {
				self.book = Some(Rc::new(resp.media));
				*self.progress.lock().unwrap() = resp.progress;
				// TODO: Check to see if we have progress. If so, generate those pages first.
				ctx.link().send_message(Msg::SendGetChapters(0, 3));
			}

			Msg::SendGetChapters(start, end) => {
				let book_id = self.book.as_ref().unwrap().id;

				ctx.link()
				.send_future(async move {
					Msg::RetrievePages(request::get_book_pages(book_id, start, end).await)
				});

				return false;
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(book) = self.book.as_ref() {
			let is_y_full_screen = self.book_dimensions.1.is_none();

			let (width, height) = (
				self.book_dimensions.0.unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_width().max(0)),
				self.book_dimensions.1.unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_height().max(0)),
			);

			// TODO: Loading screen until all chapters have done initial generation.

			let book_class = if is_y_full_screen {
				"book overlay-tools"
			}  else {
				"book"
			};

			html! {
				<div class="reading-container">
					<div class={book_class} ref={self.ref_book_container.clone()}>
						{
							if let Some(visible) = self.sidebar_visible {
								match visible {
									SidebarType::Notes => html! { <Notes book={Rc::clone(book)} /> },
									SidebarType::Settings => html! {
										<div class="settings">
											<div>
												<input value={width.to_string()} ref={self.ref_width_input.clone()} type="number" />
												<span>{ "x" }</span>
												<input value={height.to_string()} ref={self.ref_height_input.clone()} type="number" />
											</div>
											<button onclick={ctx.link().callback(|_| Msg::UpdateDimensions)}>{"Update Dimensions"}</button>
											<div>
												// TODO: Specify based on book type. Epub/Mobi (Single, Double) - PDF (Scroll)
												<select onchange={
													ctx.link()
													.callback(|e: Event| Msg::OnChangeSelection(
														e.target().unwrap()
														.unchecked_into::<web_sys::HtmlSelectElement>()
														.value()
														.parse::<u8>().unwrap()
														.into()
													))
												}>
													<option value="0" selected={self.book_display == ChapterDisplay::SinglePage}>{ "Single Page" }</option>
													<option value="1" selected={self.book_display == ChapterDisplay::DoublePage}>{ "Double Page" }</option>
												</select>
											</div>
										</div>
									},
								}
							} else {
								html! {}
							}
						}
						<div class="tools">
							<div class="tool-item" title="Open/Close the Notebook" onclick={ctx.link().callback(|_| Msg::ToggleSidebar(SidebarType::Notes))}>{ "📝" }</div>
							<div class="tool-item" title="Open/Close the Settings" onclick={ctx.link().callback(|_| Msg::ToggleSidebar(SidebarType::Settings))}>{ "⚙️" }</div>
						</div>
						<Reader
							display={self.book_display}
							progress={Rc::clone(&self.progress)}
							book={Rc::clone(book)}
							chapters={Rc::clone(&self.chapters)}
							dimensions={(width, height)}
							on_chapter_request={ctx.link().callback(|(s, e)| Msg::SendGetChapters(s, e))}
						/>
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
		if first_render {
			let id = ctx.props().id;

			ctx.link().send_future(async move {
				Msg::RetrieveBook(request::get_book_info(id).await)
			});
		}
	}
}

impl ReadingBook {
}