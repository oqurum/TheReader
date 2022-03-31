// TODO: Handle resizing.
// TODO: Allow custom sizes.

use std::{collections::{HashMap, hash_map::Entry}, rc::Rc, sync::Mutex};

use books_common::{MediaItem, Chapter, api::{GetBookIdResponse, GetChaptersResponse}, Progression};
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}};
use web_sys::HtmlIFrameElement;
use yew::{prelude::*, html::Scope};

use crate::request;
use crate::components::reader::Reader;
use crate::components::notes::Notes;


pub struct GeneratePage {
	iframe: HtmlIFrameElement,
	chapter: Chapter,
}


pub struct ChapterContents {
	#[allow(dead_code)]
	on_load: Closure<dyn FnMut()>,
	pub chapter: usize,
	pub page_count: usize,
	pub iframe: HtmlIFrameElement,
	pub is_generated: bool
}

impl ChapterContents {
	pub fn set_page(&self, page_number: usize) {
		let body = self.iframe.content_document().unwrap().body().unwrap();
		body.style().set_property("left", &format!("calc(-{}% - {}px)", 100 * page_number, page_number * 10)).unwrap();
	}
}

impl PartialEq for ChapterContents {
	fn eq(&self, other: &Self) -> bool {
		self.chapter == other.chapter
	}
}


pub struct FoundChapterPage<'a> {
	pub chapter: &'a ChapterContents,
	pub local_page: usize
}

impl<'a> PartialEq for FoundChapterPage<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.chapter == other.chapter
	}
}


pub enum Msg {
	// Event
	GenerateIFrameLoaded(GeneratePage),
	ToggleNotesVisibility,

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
	progress: Option<Progression>,
	book: Option<Rc<MediaItem>>,
	chapters: Rc<Mutex<HashMap<usize, ChapterContents>>>,
	last_grabbed_count: usize,
	// TODO: Cache pages

	book_dimensions: Option<(i32, i32)>,

	notes_visible: bool
}

impl Component for ReadingBook {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			chapters: Rc::new(Mutex::new(HashMap::new())),
			last_grabbed_count: 0,
			progress: None,
			book: None,

			book_dimensions: Some((1040, 548)),

			notes_visible: false
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::ToggleNotesVisibility => {
				self.notes_visible = !self.notes_visible;
			}

			// TODO: Add checks to see if all pages have been initially loaded.
			Msg::GenerateIFrameLoaded(page) => {
				js_update_pages_with_inlined_css(&page.iframe);

				let page_count = get_iframe_page_count(&page.iframe).max(1);

				let mut chaps = self.chapters.lock().unwrap();

				if let Entry::Occupied(mut v) = chaps.entry(page.chapter.value) {
					let chap = v.get_mut();
					chap.page_count = page_count;
					chap.is_generated = true;
				}

				let chaps_generated = chaps.values().filter(|v| v.is_generated).count();
				self.last_grabbed_count = self.last_grabbed_count.saturating_sub(1);

				drop(chaps);

				if chaps_generated == self.book.as_ref().unwrap().chapter_count {
					self.on_all_frames_generated();
				} else if self.last_grabbed_count == 0 {
					self.update_chapter_pages();
					ctx.link().send_message(Msg::SendGetChapters(chaps_generated, chaps_generated + 3));
				}
			}

			Msg::RetrievePages(info) => {
				self.last_grabbed_count = info.limit;
				// Reverse iterator since for some reason chapter "generation" works from LIFO
				for chap in info.chapters.into_iter().rev() {
					log::info!("Generating Chapter {}", chap.value + 1);
					self.chapters.lock().unwrap().insert(chap.value, generate_pages(self.book_dimensions, chap, ctx.link().clone()));
				}
			}

			Msg::RetrieveBook(resp) => {
				self.book = Some(Rc::new(resp.media));
				self.progress = resp.progress;
				// TODO: Check to see if we have progress. If so, generate those pages first.
				ctx.link().send_message(Msg::SendGetChapters(0, 3));
			}

			Msg::SendGetChapters(start, end) => {
				let book_id = self.book.as_ref().unwrap().id;

				ctx.link()
				.send_future(async move {
					Msg::RetrievePages(request::get_book_pages(book_id, start, end).await)
				});
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(book) = self.book.as_ref() {
			let (width, height) = match self.book_dimensions { // TODO: Use Option.unzip once stable.
				Some((a, b)) => (a, b),
				None => (gloo_utils::body().client_width().max(0), gloo_utils::body().client_height().max(0)),
			};

			// TODO: Loading screen until all chapters have done initial generation.

			html! {
				<div class="reading-container">
					<div class="book">
						<Notes visible={self.notes_visible} book={Rc::clone(book)} />
						<div class="tools">
							<div class="tool-item" title="Open/Close the Notebook" onclick={ctx.link().callback(|_| Msg::ToggleNotesVisibility)}>{ "📝" }</div>
							// <div class="tool-item" title="Book Summary">{ "📝" }</div>
						</div>
						<Reader
							progress={self.progress.filter(|_| self.have_all_chapters_passed_init_generation())}
							book={Rc::clone(book)}
							chapters={Rc::clone(&self.chapters)}
							width={width}
							height={height}
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
	fn update_chapter_pages(&mut self) {
		for ele in self.chapters.lock().unwrap().values_mut() {
			ele.page_count = get_iframe_page_count(&ele.iframe).max(1);
		}
	}

	fn on_all_frames_generated(&mut self) {
		log::info!("All Frames Generated");
		// Double check page counts before proceeding.
		self.update_chapter_pages();

		// TODO: Remove .filter from fn view Reader progress. Replace with event.
	}

	fn have_all_chapters_passed_init_generation(&self) -> bool {
		let chaps_generated = self.chapters.lock().unwrap().values().filter(|v| v.is_generated).count();

		chaps_generated == self.book.as_ref().unwrap().chapter_count
	}
}


#[derive(Properties, PartialEq)]
pub struct Props {
    pub html: String,
}

#[function_component(SafeHtml)]
pub fn safe_html(props: &Props) -> Html {
    let div = gloo_utils::document().create_element("div").unwrap();
    div.set_inner_html(&props.html.clone());

    Html::VRef(div.into())
}

fn create_iframe() -> HtmlIFrameElement {
	gloo_utils::document()
		.create_element("iframe")
		.unwrap()
		.dyn_into()
		.unwrap()
}

fn generate_pages(book_dimensions: Option<(i32, i32)>, chapter: Chapter, scope: Scope<ReadingBook>) -> ChapterContents {
	let iframe = create_iframe();
	iframe.set_attribute("srcdoc", chapter.html.as_str()).unwrap();

	{
		let (width, height) = match book_dimensions { // TODO: Use Option.unzip once stable.
			Some((a, b)) => (a, b),
			None => (gloo_utils::body().client_width().max(0), gloo_utils::body().client_height().max(0)),
		};

		iframe.style().set_property("width", &format!("{}px", width)).unwrap();
		iframe.style().set_property("height", &format!("{}px", height)).unwrap();
	}

	let new_frame = iframe.clone();

	let chap_value = chapter.value;

	let f = Closure::wrap(Box::new(move || {
		scope.send_message(Msg::GenerateIFrameLoaded(GeneratePage {
			iframe: iframe.clone(),
			chapter: chapter.clone()
		}));
	}) as Box<dyn FnMut()>);

	new_frame.set_onload(Some(f.as_ref().unchecked_ref()));

	ChapterContents {
		page_count: 1,
		chapter: chap_value,
		iframe: new_frame,
		on_load: f,
		is_generated: false
	}
}



#[derive(Clone, Copy)]
pub enum PageDisplay {
	SinglePage = 0,
	DoublePage,
	// VerticalPage,
}


#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
	// TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
	fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;
	fn js_update_pages_with_inlined_css(iframe: &HtmlIFrameElement);
	fn js_set_page_display_style(iframe: &HtmlIFrameElement, display: usize);
}