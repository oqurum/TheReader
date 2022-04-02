use std::{collections::{HashMap, hash_map::Entry}, rc::Rc, sync::Mutex, path::PathBuf};

use books_common::{MediaItem, Progression, Chapter};
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}};
use web_sys::HtmlIFrameElement;
use yew::{prelude::*, html::Scope};

use crate::request;



#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
	// TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
	fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;

	fn js_get_current_byte_pos(iframe: &HtmlIFrameElement) -> Option<usize>;
	fn js_get_page_from_byte_position(iframe: &HtmlIFrameElement, position: usize) -> Option<usize>;

	fn js_update_iframe_after_load(iframe: &HtmlIFrameElement, chapter: usize, handle_js_redirect_clicks: &Closure<dyn FnMut(usize, String)>);
	fn js_set_page_display_style(iframe: &HtmlIFrameElement, display: u8);
}



struct CachedPage {
	chapter: usize,
	chapter_local_page: usize,
}

struct CachedChapter {
	/// Page index that this chapter starts at.
	index: usize,
	/// Total Pages inside chapter
	pages: usize,
}


// Currently used to load in chapters to the Reader.
pub struct LoadedChapters {
	pub total: usize,
	pub chapters: Vec<Chapter>,
}

impl LoadedChapters {
	pub fn new() -> Self {
		Self {
			total: 0,
			chapters: Vec::new(),
		}
	}
}


#[derive(Properties)]
pub struct Property {
	// Callbacks
	pub on_chapter_request: Callback<(usize, usize)>,

	pub book: Rc<MediaItem>,
	pub chapters: Rc<Mutex<LoadedChapters>>,

	pub progress: Option<Progression>,
	pub display: ChapterDisplay,

	pub dimensions: (i32, i32),
	// pub ratio: (usize, usize)
}


impl PartialEq for Property {
	fn eq(&self, _other: &Self) -> bool {
		// TODO
		false
	}
}


pub enum TouchMsg {
	Start(i32, i32),
	End(i32, i32),
	Cancel
}


pub enum Msg {
	GenerateIFrameLoaded(GenerateChapter),

	// Event
	HandleJsRedirect(usize, String, Option<String>),

	Touch(TouchMsg),
	NextPage,
	PreviousPage,
	SetPage(usize),
	Ignore
}


pub struct Reader {
	// Cached from External Source
	cached_display: ChapterDisplay,
	cached_dimensions: Option<(i32, i32)>,
	generated_chapters: HashMap<usize, ChapterContents>,

	/// All pages which we've registered.
	cached_pages: Vec<CachedPage>,
	/// Position of the chapter based off the page index.
	cached_chapter_pos: Vec<CachedChapter>,

	/// The Total page we're currently on
	total_page_position: usize,

	/// The Chapter we're in
	viewing_chapter: usize,
	// TODO: Decide if we want to keep. Not really needed since we can aquire it based off of self.cached_pages[self.total_page_position].chapter

	handle_js_redirect_clicks: Closure<dyn FnMut(usize, String)>,

	handle_touch_start: Option<Closure<dyn FnMut(TouchEvent)>>,
	handle_touch_end: Option<Closure<dyn FnMut(TouchEvent)>>,
	handle_touch_cancel: Option<Closure<dyn FnMut(TouchEvent)>>,

	touch_start: Option<(i32, i32)>
}

impl Component for Reader {
	type Message = Msg;
	type Properties = Property;

	fn create(ctx: &Context<Self>) -> Self {
		let link = ctx.link().clone();
		let handle_js_redirect_clicks = Closure::wrap(Box::new(move |chapter: usize, path: String| {
			let (file_path, id_value) = path.split_once('#')
				.map(|(a, b)| (a.to_string(), Some(b.to_string())))
				.unwrap_or((path, None));

			link.send_message(Msg::HandleJsRedirect(chapter, file_path, id_value));
		}) as Box<dyn FnMut(usize, String)>);

		Self {
			cached_display: ChapterDisplay::DoublePage,
			cached_dimensions: None,
			generated_chapters: HashMap::new(),

			cached_pages: Vec::new(),
			cached_chapter_pos: Vec::new(),

			total_page_position: 0,
			viewing_chapter: 0, // TODO: Add after "frames loaded" event. // ctx.props().progress.map(|v| match v { Progression::Ebook { chapter, page } => chapter as usize, _ => 0 }).unwrap_or_default(),

			handle_js_redirect_clicks,

			handle_touch_cancel: None,
			handle_touch_end: None,
			handle_touch_start: None,

			touch_start: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Ignore => return false,

			Msg::HandleJsRedirect(_chapter, file_path, _id_name) => {
				let file_path = PathBuf::from(file_path);

				let chaps = ctx.props().chapters.lock().unwrap();

				// TODO: Ensure we handle any paths which go to a parent directory. eg: "../file.html"
				// let mut path = chaps.chapters.iter().find(|v| v.value == chapter).unwrap().file_path.clone();
				// path.pop();

				if let Some(chap) = chaps.chapters.iter().find(|v| v.file_path == file_path) {
					self.set_chapter(chap.value);
					// TODO: Handle id_name
				}
			}

			Msg::SetPage(new_page) => {
				return self.set_page(new_page.min(self.page_count().saturating_sub(1)), ctx);
			}

			Msg::NextPage => {
				if self.total_page_position + 1 == self.page_count() {
					return false;
				}

				// Double check all chapters if we're currently switching from first page.
				// if self.total_page_position == 0 {
				// 	self.update_cached_pages();
				// }

				self.set_page(self.total_page_position + 1, ctx);
			}

			Msg::PreviousPage => {
				if self.total_page_position == 0 {
					return false;
				}

				self.set_page(self.total_page_position - 1, ctx);
			}

			Msg::Touch(msg) => match msg {
				TouchMsg::Start(point_x, point_y) => {
					self.touch_start = Some((point_x, point_y));
					return false;
				}

				TouchMsg::End(end_x, end_y) => {
					if let Some((start_x, start_y)) = self.touch_start {
						let (dist_x, dist_y) = (start_x - end_x, start_y - end_y);

						// Are we dragging vertically or horizontally?
						if dist_x.abs() > dist_y.abs() {
							if dist_x.abs() > 100 {
								log::info!("Changing Page");

								if dist_x.is_positive() {
									log::info!("Next Page");
									ctx.link().send_message(Msg::NextPage);
								} else {
									log::info!("Previous Page");
									ctx.link().send_message(Msg::PreviousPage);
								}
							}
						} else {
							// TODO: Vertical
						}

						self.touch_start = None;
					}

					return false;
				}

				TouchMsg::Cancel => {
					self.touch_start = None;
					return false;
				}
			}

			Msg::GenerateIFrameLoaded(page) => {
				js_update_iframe_after_load(&page.iframe, page.chapter.value, &self.handle_js_redirect_clicks);

				if let Entry::Occupied(mut v) = self.generated_chapters.entry(page.chapter.value) {
					let chap = v.get_mut();
					chap.is_generated = true;
				}

				let chaps_generated = self.generated_chapters.values().filter(|v| v.is_generated).count();

				if chaps_generated == ctx.props().book.chapter_count {
					self.on_all_frames_generated();
				} else if chaps_generated == self.generated_chapters.len() {
					// self.update_cached_pages();
					ctx.props().on_chapter_request.emit((chaps_generated, chaps_generated + 3));
				}
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		// let node = self.view_page(ctx);
		let page_count = self.page_count();
		let frames = self.get_frames(ctx.props());

		let pages_style = format!("width: {}px; height: {}px;", ctx.props().dimensions.0, ctx.props().dimensions.1);

		let progress_precentage = format!("width: {}%;", (self.total_page_position + 1) as f64 / page_count as f64 * 100.0);

		html! {
			<div class="reader">
				<div class="navbar">
					<a onclick={ctx.link().callback(|_| Msg::SetPage(0))}>{"First Page"}</a>
					<a onclick={ctx.link().callback(|_| Msg::PreviousPage)}>{"Previous Page"}</a>
					<span>{self.total_page_position + 1} {"/"} {page_count} {" pages"}</span>
					<a onclick={ctx.link().callback(|_| Msg::NextPage)}>{"Next Page"}</a>
					<a onclick={ctx.link().callback(move |_| Msg::SetPage(page_count - 1))}>{"Last Page"}</a>
				</div>

				<div class="pages" style={pages_style}>
					<div class="frames" style={format!("top: -{}%;", self.viewing_chapter * 100)}>
						{ for frames.into_iter().map(|v| Html::VRef(v.into())) }
					</div>
				</div>
				<div class="progress"><div class="prog-bar" style={progress_precentage}></div></div>
			</div>
		}
	}

	fn changed(&mut self, ctx: &Context<Self>) -> bool {
		let props = ctx.props();

		if self.cached_display != props.display || self.cached_dimensions != Some(props.dimensions) {
			self.cached_display = props.display;
			self.cached_dimensions = Some(props.dimensions);

			for chap in self.generated_chapters.values() {
				js_set_page_display_style(&chap.iframe, props.display.into());
				update_iframe_size(Some(props.dimensions), &chap.iframe);
			}

			self.update_cached_pages();
		}


		if let Some(prog) = props.progress.filter(|_| self.have_all_chapters_passed_init_generation(props)) {
			match prog {
				Progression::Ebook { chapter, page, char_pos } if self.viewing_chapter == 0 => {
					// TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
					self.set_chapter(chapter as usize);

					if char_pos != -1 {
						let chapter = self.generated_chapters.get(&self.viewing_chapter).unwrap();

						let page = js_get_page_from_byte_position(&chapter.iframe, char_pos as usize);

						if let Some(page) = page {
							chapter.set_page(page);
						}
					}
				}

				_ => ()
			}
		} else {
			// Continue loading chapters
			let chaps = props.chapters.lock().unwrap();

			// Reverse iterator since for some reason chapter "generation" works from LIFO
			for chap in chaps.chapters.iter().rev() {
				if let Entry::Vacant(v) = self.generated_chapters.entry(chap.value) {
					log::info!("Generating Chapter {}", chap.value + 1);
					v.insert(generate_pages(Some(props.dimensions), chap.clone(), ctx.link().clone()));
				}
			}
		}

		true
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		if first_render {
			let window = gloo_utils::window();

			// TODO: Touch Start is being called 24 times at once.
			// TODO: Not working for my Samsung Tablet with Firefox.

			let link = ctx.link().clone();
			let handle_touch_start = Closure::wrap(Box::new(move |event: TouchEvent| {
				let touches = event.touches();

				if touches.length() == 1 {
					let touch = touches.get(0).unwrap();
					link.send_message(Msg::Touch(TouchMsg::Start(touch.client_x(), touch.client_y())));
				}
			}) as Box<dyn FnMut(TouchEvent)>);

			let link = ctx.link().clone();
			let handle_touch_end = Closure::wrap(Box::new(move |event: TouchEvent| {
				let touches = event.touches();

				if touches.length() == 1 {
					let touch = touches.get(0).unwrap();
					link.send_message(Msg::Touch(TouchMsg::End(touch.client_x(), touch.client_y())));
				}
			}) as Box<dyn FnMut(TouchEvent)>);

			let link = ctx.link().clone();
			let handle_touch_cancel = Closure::wrap(Box::new(move |_: TouchEvent| {
				link.send_message(Msg::Touch(TouchMsg::Cancel));
			}) as Box<dyn FnMut(TouchEvent)>);

			window.add_event_listener_with_callback("touchstart", handle_touch_start.as_ref().unchecked_ref()).unwrap();
			window.add_event_listener_with_callback("touchend", handle_touch_end.as_ref().unchecked_ref()).unwrap();
			window.add_event_listener_with_callback("touchcancel", handle_touch_cancel.as_ref().unchecked_ref()).unwrap();

			self.handle_touch_cancel = Some(handle_touch_cancel);
			self.handle_touch_end = Some(handle_touch_end);
			self.handle_touch_start = Some(handle_touch_start);
		}
	}
}

impl Reader {
	fn have_all_chapters_passed_init_generation(&self, props: &Property) -> bool {
		let chaps_generated = self.generated_chapters.values().filter(|v| v.is_generated).count();

		chaps_generated == props.book.chapter_count
	}

	fn update_cached_pages(&mut self) {
		self.cached_chapter_pos.clear();
		self.cached_pages.clear();

		let mut total_page_pos = 0;

		// TODO: Verify if needed. Or can we do values_mut() we need to have it in asc order
		for chap in 0..self.generated_chapters.len() {
			if let Some(ele) = self.generated_chapters.get_mut(&chap) {
				let page_count = get_iframe_page_count(&ele.iframe).max(1);

				self.cached_chapter_pos.push(CachedChapter {
					index: total_page_pos,
					pages: page_count,
				});

				total_page_pos += page_count;


				for i in 0..page_count {
					self.cached_pages.push(CachedPage {
						chapter: ele.chapter,
						chapter_local_page: i
					});
				}
			}
		}
	}

	fn on_all_frames_generated(&mut self) {
		log::info!("All Frames Generated");
		// Double check page counts before proceeding.
		self.update_cached_pages();

		// TODO: Remove .filter from fn view Reader progress. Replace with event.
	}

	fn set_page(&mut self, new_total_page: usize, ctx: &Context<Self>) -> bool {
		if let Some(page) = self.cached_pages.get(new_total_page) {
			self.total_page_position = new_total_page;
			self.viewing_chapter = page.chapter;

			self.generated_chapters.get(&self.viewing_chapter).unwrap().set_page(page.chapter_local_page);

			self.upload_progress(ctx);

			true
		} else {
			false
		}
	}

	fn set_chapter(&mut self, new_chapter: usize) -> bool {
		if let Some(chap) = self.cached_chapter_pos.get(new_chapter) {
			self.total_page_position = chap.index;
			self.viewing_chapter = new_chapter;

			true
		} else {
			false
		}
	}

	pub fn page_count(&self) -> usize {
		self.cached_pages.len()
	}

	pub fn get_frames(&self, props: &Property) -> Vec<HtmlIFrameElement> {
		let mut items = Vec::new();

		for i in 0..props.book.chapter_count {
			if let Some(v) = self.generated_chapters.get(&i) {
				items.push(v.iframe.clone());
			} else {
				break;
			}
		}

		items
	}

	fn upload_progress(&self, ctx: &Context<Self>) {
		let (chapter, page, char_pos, book_id) = (
			self.viewing_chapter as i64,
			self.total_page_position as i64,
			js_get_current_byte_pos(&self.generated_chapters.get(&self.viewing_chapter).unwrap().iframe).map(|v| v as i64).unwrap_or(-1),
			ctx.props().book.id
		);

		let last_page = self.page_count().saturating_sub(1);

		ctx.link()
		.send_future(async move {
			match page {
				0 => {
					request::remove_book_progress(book_id).await;
				}

				// TODO: Figure out what the last page of each book actually is.
				v if v as usize == last_page => {
					request::update_book_progress(book_id, &Progression::Complete).await;
				}

				_ => {
					let progression = Progression::Ebook {
						char_pos,
						chapter,
						page
					};

					request::update_book_progress(book_id, &progression).await;
				}
			}

			Msg::Ignore
		});
	}
}




#[derive(Clone, Copy, PartialEq)]
pub enum ChapterDisplay {
	SinglePage = 0,
	DoublePage,
	// VerticalPage,
}

impl From<u8> for ChapterDisplay {
	fn from(value: u8) -> Self {
		match value {
			0 => Self::SinglePage,
			1 => Self::DoublePage,
			_ => unimplemented!()
		}
	}
}


impl From<ChapterDisplay> for u8 {
	fn from(val: ChapterDisplay) -> Self {
		val as u8
	}
}


fn create_iframe() -> HtmlIFrameElement {
	gloo_utils::document()
		.create_element("iframe")
		.unwrap()
		.dyn_into()
		.unwrap()
}

fn generate_pages(book_dimensions: Option<(i32, i32)>, chapter: Chapter, scope: Scope<Reader>) -> ChapterContents {
	let iframe = create_iframe();
	iframe.set_attribute("srcdoc", chapter.html.as_str()).unwrap();

	update_iframe_size(book_dimensions, &iframe);

	let new_frame = iframe.clone();

	let chap_value = chapter.value;

	let f = Closure::wrap(Box::new(move || {
		scope.send_message(Msg::GenerateIFrameLoaded(GenerateChapter {
			iframe: iframe.clone(),
			chapter: chapter.clone()
		}));
	}) as Box<dyn FnMut()>);

	new_frame.set_onload(Some(f.as_ref().unchecked_ref()));

	ChapterContents {
		chapter: chap_value,
		iframe: new_frame,
		on_load: f,
		is_generated: false
	}
}

fn update_iframe_size(book_dimensions: Option<(i32, i32)>, iframe: &HtmlIFrameElement) {
	let (width, height) = match book_dimensions { // TODO: Use Option.unzip once stable.
		Some((a, b)) => (a, b),
		None => (gloo_utils::body().client_width().max(0), gloo_utils::body().client_height().max(0)),
	};

	iframe.style().set_property("width", &format!("{}px", width)).unwrap();
	iframe.style().set_property("height", &format!("{}px", height)).unwrap();
}

pub struct GenerateChapter {
	iframe: HtmlIFrameElement,
	chapter: Chapter,
}


pub struct ChapterContents {
	#[allow(dead_code)]
	on_load: Closure<dyn FnMut()>,
	pub chapter: usize,
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