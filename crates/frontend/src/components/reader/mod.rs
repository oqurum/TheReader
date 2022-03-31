use std::{collections::{HashMap, hash_map::Entry}, rc::Rc, sync::Mutex};

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

	fn js_update_pages_with_inlined_css(iframe: &HtmlIFrameElement);
	fn js_set_page_display_style(iframe: &HtmlIFrameElement, display: usize);
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

	pub width: i32,
	pub height: i32,
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
	Touch(TouchMsg),
	NextPage,
	PreviousPage,
	SetPage(usize),
	Ignore
}


pub struct Reader {
	pub generated_chapters: HashMap<usize, ChapterContents>,

	total_page_position: usize,

	viewing_chapter: usize,
	viewing_chapter_page: usize,

	handle_touch_start: Option<Closure<dyn FnMut(TouchEvent)>>,
	handle_touch_end: Option<Closure<dyn FnMut(TouchEvent)>>,
	handle_touch_cancel: Option<Closure<dyn FnMut(TouchEvent)>>,

	touch_start: Option<(i32, i32)>
}

impl Component for Reader {
	type Message = Msg;
	type Properties = Property;

	fn create(ctx: &Context<Self>) -> Self {
		Self {
			generated_chapters: HashMap::new(),
			total_page_position: 0,
			viewing_chapter: 0, // TODO: Add after "frames loaded" event. // ctx.props().progress.map(|v| match v { Progression::Ebook { chapter, page } => chapter as usize, _ => 0 }).unwrap_or_default(),
			viewing_chapter_page: 0,

			handle_touch_cancel: None,
			handle_touch_end: None,
			handle_touch_start: None,

			touch_start: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Ignore => return false,

			Msg::SetPage(new_page) => {
				return self.set_page(new_page, ctx);
			}

			Msg::NextPage => {
				if self.total_page_position + 1 == self.page_count() {
					return false;
				}

				let curr_chap_page_count = self.get_chapter_page_count(self.viewing_chapter, &self.generated_chapters);


				// Double check all chapters if we're currently switching from first page.
				if self.total_page_position == 0 {
					self.generated_chapters.values_mut()
					.for_each(|chap| chap.page_count = get_iframe_page_count(&chap.iframe).max(1));
				}

				if let Some(curr_chap_page_count) = curr_chap_page_count {
					// Same Chapter?
					if self.viewing_chapter_page + 1 < curr_chap_page_count {
						// Increment relative page count
						self.viewing_chapter_page += 1;
						self.total_page_position += 1;

						self.generated_chapters.get(&self.viewing_chapter).unwrap().set_page(self.viewing_chapter_page);
					}

					// Go to next chapter
					else if self.generated_chapters.contains_key(&(self.viewing_chapter + 1)) {
						self.viewing_chapter_page = 0;
						self.viewing_chapter += 1;
						self.total_page_position += 1;
					}
				}

				let (chapter, page, char_pos, book_id) = (
					self.viewing_chapter as i64,
					self.total_page_position as i64,
					js_get_current_byte_pos(&self.generated_chapters.get(&self.viewing_chapter).unwrap().iframe).map(|v| v as i64).unwrap_or(-1),
					ctx.props().book.id
				);

				ctx.link()
				.send_future(async move {
					let progression = Progression::Ebook {
						char_pos,
						chapter,
						page
					};

					request::update_book_progress(book_id, &progression).await;

					Msg::Ignore
				});
			}

			Msg::PreviousPage => {
				if self.total_page_position == 0 {
					return false;
				}

				// Current chapter still.
				if self.viewing_chapter_page != 0 {
					self.viewing_chapter_page -= 1;
					self.total_page_position -= 1;

					self.generated_chapters.get(&self.viewing_chapter).unwrap().set_page(self.viewing_chapter_page);
				}

				// Previous chapter.
				else if self.generated_chapters.contains_key(&(self.viewing_chapter - 1)) {
					self.viewing_chapter -= 1;
					self.total_page_position -= 1;

					self.viewing_chapter_page = self.get_chapter_page_count(self.viewing_chapter, &self.generated_chapters).unwrap() - 1;

					self.generated_chapters.get(&self.viewing_chapter).unwrap().set_page(self.viewing_chapter_page);
				}

				let (chapter, page, char_pos, book_id) = (
					self.viewing_chapter as i64,
					self.total_page_position as i64,
					js_get_current_byte_pos(&self.generated_chapters.get(&self.viewing_chapter).unwrap().iframe).map(|v| v as i64).unwrap_or(-1),
					ctx.props().book.id
				);

				ctx.link()
				.send_future(async move {
					if page == 0 {
						request::remove_book_progress(book_id).await;
					} else {
						let progression = Progression::Ebook {
							char_pos,
							chapter,
							page
						};

						request::update_book_progress(book_id, &progression).await;
					}

					Msg::Ignore
				});
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
				js_update_pages_with_inlined_css(&page.iframe);

				let page_count = get_iframe_page_count(&page.iframe).max(1);

				if let Entry::Occupied(mut v) = self.generated_chapters.entry(page.chapter.value) {
					let chap = v.get_mut();
					chap.page_count = page_count;
					chap.is_generated = true;
				}

				let chaps_generated = self.generated_chapters.values().filter(|v| v.is_generated).count();

				if chaps_generated == ctx.props().book.chapter_count {
					self.on_all_frames_generated(ctx.props());
				} else if chaps_generated == self.generated_chapters.len() {
					self.update_chapter_pages();
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

		let pages_style = format!("width: {}px; height: {}px;", ctx.props().width, ctx.props().height);

		let progress_precentage = format!("width: {}%;", (self.total_page_position + 1) as f64 / page_count as f64 * 100.0);

		html! {
			<div class="reader">
				<div class="navbar">
					<a onclick={ctx.link().callback(|_| Msg::SetPage(0))}>{"First Page"}</a>
					<a onclick={ctx.link().callback(|_| Msg::PreviousPage)}>{"Previous Page"}</a>
					<span>{self.total_page_position + 1} {"/"} {page_count} {" pages"}</span>
					<a onclick={ctx.link().callback(|_| Msg::NextPage)}>{"Next Page"}</a>
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
		log::info!("changed");

		if let Some(prog) = ctx.props().progress.filter(|_| self.have_all_chapters_passed_init_generation(ctx.props())) {
			match prog {
				Progression::Ebook { chapter, page, char_pos } if self.viewing_chapter == 0 => {
					// TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
					self.set_chapter(chapter as usize, ctx);

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
			let props = ctx.props();
			let chaps = props.chapters.lock().unwrap();

			// Reverse iterator since for some reason chapter "generation" works from LIFO
			for chap in chaps.chapters.iter().rev() {
				if let Entry::Vacant(v) = self.generated_chapters.entry(chap.value) {
					log::info!("Generating Chapter {}", chap.value + 1);
					v.insert(generate_pages(Some((props.width, props.height)), chap.clone(), ctx.link().clone()));
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

	fn update_chapter_pages(&mut self) {
		for ele in self.generated_chapters.values_mut() {
			ele.page_count = get_iframe_page_count(&ele.iframe).max(1);
		}
	}

	fn on_all_frames_generated(&mut self, props: &Property) {
		log::info!("All Frames Generated");
		// Double check page counts before proceeding.
		self.update_chapter_pages();

		// TODO: Remove .filter from fn view Reader progress. Replace with event.
	}

	fn get_chapter_page_count(&self, chapter: usize, chapters: &HashMap<usize, ChapterContents>) -> Option<usize> {
		Some(chapters.get(&chapter)?.page_count)
	}

	fn find_page<'a>(&self, total_page_position: usize, chapter_count: usize, chapters: &'a HashMap<usize, ChapterContents>) -> Option<FoundChapterPage<'a>> {
		let mut curr_page = 0;

		for chap_number in 0..chapter_count {
			let chapter = chapters.get(&chap_number)?;

			let local_page = total_page_position - curr_page;

			if local_page < chapter.page_count {
				return Some(FoundChapterPage {
					chapter,
					local_page
				});
			}

			curr_page += chapter.page_count;
		}

		None
	}

	fn set_page(&mut self, new_total_page: usize, ctx: &Context<Self>) -> bool {
		if new_total_page == self.page_count() || self.total_page_position == new_total_page {
			false
		} else {
			let chapter_count = ctx.props().book.chapter_count;

			let both_pages = self.find_page(self.total_page_position, chapter_count, &self.generated_chapters)
				.zip(self.find_page(new_total_page, chapter_count, &self.generated_chapters));

			if let Some((c1, c2)) = both_pages {
				if c1 == c2 {
					c2.chapter.set_page(c2.local_page);
				} else {
					self.viewing_chapter = c2.chapter.chapter;
				}
			}

			self.total_page_position = new_total_page;

			true
		}
	}

	fn set_chapter(&mut self, new_chapter: usize, ctx: &Context<Self>) -> bool {
		let chapter_count = ctx.props().book.chapter_count;

		if self.viewing_chapter == new_chapter || new_chapter > chapter_count {
			false
		} else {
			self.total_page_position = 0;
			self.viewing_chapter_page = 0;
			self.viewing_chapter = new_chapter;


			for i in 0..=new_chapter {
				if let Some(chap) = self.generated_chapters.get(&i) {
					if i != new_chapter {
						self.total_page_position += chap.page_count;
					}
				} else {
					self.total_page_position = 0;
					return false;
				}
			}

			true
		}
	}

	pub fn page_count(&self) -> usize {
		let mut pages = 0;

		self.generated_chapters.values().for_each(|v| pages += v.page_count);

		pages
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
}




#[derive(Clone, Copy)]
pub enum PageDisplay {
	SinglePage = 0,
	DoublePage,
	// VerticalPage,
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
		scope.send_message(Msg::GenerateIFrameLoaded(GenerateChapter {
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

pub struct GenerateChapter {
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