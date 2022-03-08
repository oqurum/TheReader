// TODO: Handle resizing.
// TODO: Allow custom sizes.
// TODO: Batch chapter grabs. Add progress bar at bottom of page.

use std::{collections::{HashMap, hash_map::Entry}, rc::Rc, sync::Mutex};

use books_common::{MediaItem, Chapter};
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}};
use web_sys::HtmlIFrameElement;
use yew::{prelude::*, html::Scope};

use crate::fetch;
use crate::components::reader::Reader;
use crate::components::notes::Notes;


pub enum PageDisplay {
	SinglePage,
	DoublePage,
	CustomAmount(usize)
}



#[derive(serde::Deserialize)]
pub struct ChapterInfo {
	chapters: Vec<Chapter>
}


pub struct GeneratePage {
	iframe: HtmlIFrameElement,
	chapter: Chapter,
}


pub struct ChapterContents {
	#[allow(dead_code)]
	on_load: Closure<dyn FnMut()>,
	pub chapter: usize,
	pub page_count: usize,
	pub iframe: HtmlIFrameElement
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
	RetrieveBook(MediaItem),
	RetrievePages(ChapterInfo),
}

#[derive(Properties, PartialEq)]
pub struct Property {
	pub id: usize
}

pub struct ReadingBook {
	book: Option<Rc<MediaItem>>,
	chapters: Rc<Mutex<HashMap<usize, ChapterContents>>>,
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

				if let Entry::Occupied(mut v) = self.chapters.lock().unwrap().entry(page.chapter.value) {
					v.get_mut().page_count = page_count;
				}
			}

			Msg::RetrievePages(info) => {
				for chap in info.chapters {
					log::info!("Generating Chapter {}", chap.value + 1);
					self.chapters.lock().unwrap().insert(chap.value, generate_pages(self.book_dimensions, chap, ctx.link().clone()));
				}
			}

			Msg::RetrieveBook(book) => {
				self.book = Some(Rc::new(book));

				ctx.link().send_message(Msg::SendGetChapters(0, self.book.as_ref().unwrap().chapter_count));
			}

			Msg::SendGetChapters(start, end) => {
				let book_id = self.book.as_ref().unwrap().id;

				ctx.link()
				.send_future(async move {
					Msg::RetrievePages(fetch("GET", &format!("/api/book/{}/pages/{}-{}", book_id, start, end), Option::<&()>::None).await.unwrap())
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

			html! {
				<div class="reading-container">
					<div class="book">
						<Notes visible={self.notes_visible} />
						<div class="tools">
							<div class="tool-item" title="Open/Close the Notebook" onclick={ctx.link().callback(|_| Msg::ToggleNotesVisibility)}>{ "📝" }</div>
							// <div class="tool-item" title="Book Summary">{ "📝" }</div>
						</div>
						<Reader
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
		self.update_chapter_pages();

		if first_render {
			let id = ctx.props().id;

			ctx.link().send_future(async move {
				Msg::RetrieveBook(fetch("GET", &format!("/api/book/{}", id), Option::<&()>::None).await.unwrap())
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
		on_load: f
	}
}


#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
	// TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
	fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;
	fn js_update_pages_with_inlined_css(iframe: &HtmlIFrameElement);
}