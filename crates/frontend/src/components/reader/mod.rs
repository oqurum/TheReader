// Move reading.rs into here. Modify reading.rs to utilize this.
// Add dimensions into Property.

use std::{collections::HashMap, rc::Rc, sync::Mutex};

use books_common::{MediaItem, Progression};
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}};
use web_sys::HtmlIFrameElement;
use yew::prelude::*;

use crate::{pages::reading::{ChapterContents, FoundChapterPage}, request};

#[derive(Properties)]
pub struct Property {
	pub book: Rc<MediaItem>,
	pub chapters: Rc<Mutex<HashMap<usize, ChapterContents>>>,

	pub progress: Option<Progression>,

	pub width: i32,
	pub height: i32,
	// pub ratio: (usize, usize)
}

impl Property {
	pub fn page_count(&self) -> usize {
		let mut pages = 0;

		let chapters = self.chapters.lock().unwrap();

		chapters.values().for_each(|v| pages += v.page_count);

		pages
	}

	pub fn get_frames(&self) -> Vec<HtmlIFrameElement> {
		let mut items = Vec::new();

		let chapters = self.chapters.lock().unwrap();

		for i in 0..self.book.chapter_count {
			if let Some(v) = chapters.get(&i) {
				items.push(v.iframe.clone());
			} else {
				break;
			}
		}

		items
	}
}

impl PartialEq for Property {
	fn eq(&self, _other: &Self) -> bool {
		false
	}
}


pub enum TouchMsg {
	Start(i32, i32),
	End(i32, i32),
	Cancel
}


pub enum Msg {
	// Event
	Touch(TouchMsg),
	NextPage,
	PreviousPage,
	SetPage(usize),
	Ignore
}


pub struct Reader {
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
				if self.total_page_position + 1 == ctx.props().page_count() {
					return false;
				}

				let mut chapters = ctx.props().chapters.lock().unwrap();

				let curr_chap_page_count = self.get_chapter_page_count(self.viewing_chapter, &*chapters);


				// Double check all chapters if we're currently switching from first page.
				if self.total_page_position == 0 {
					chapters.values_mut()
					.for_each(|chap| chap.page_count = get_iframe_page_count(&chap.iframe).max(1));
				}

				if let Some(curr_chap_page_count) = curr_chap_page_count {
					// Same Chapter?
					if self.viewing_chapter_page + 1 < curr_chap_page_count {
						// Increment relative page count
						self.viewing_chapter_page += 1;
						self.total_page_position += 1;

						chapters.get(&self.viewing_chapter).unwrap().set_page(self.viewing_chapter_page);
					}

					// Go to next chapter
					else if chapters.contains_key(&(self.viewing_chapter + 1)) {
						self.viewing_chapter_page = 0;
						self.viewing_chapter += 1;
						self.total_page_position += 1;
					}
				}

				let (chapter, page, char_pos, book_id) = (
					self.viewing_chapter as i64,
					self.total_page_position as i64,
					js_get_current_byte_pos(&chapters.get(&self.viewing_chapter).unwrap().iframe).map(|v| v as i64).unwrap_or(-1),
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

				let chapters = ctx.props().chapters.lock().unwrap();

				// Current chapter still.
				if self.viewing_chapter_page != 0 {
					self.viewing_chapter_page -= 1;
					self.total_page_position -= 1;

					chapters.get(&self.viewing_chapter).unwrap().set_page(self.viewing_chapter_page);
				}

				// Previous chapter.
				else if chapters.contains_key(&(self.viewing_chapter - 1)) {
					self.viewing_chapter -= 1;
					self.total_page_position -= 1;

					self.viewing_chapter_page = self.get_chapter_page_count(self.viewing_chapter, &*chapters).unwrap() - 1;

					chapters.get(&self.viewing_chapter).unwrap().set_page(self.viewing_chapter_page);
				}

				let (chapter, page, char_pos, book_id) = (
					self.viewing_chapter as i64,
					self.total_page_position as i64,
					js_get_current_byte_pos(&chapters.get(&self.viewing_chapter).unwrap().iframe).map(|v| v as i64).unwrap_or(-1),
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
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		// let node = self.view_page(ctx);
		let page_count = ctx.props().page_count();
		let frames = ctx.props().get_frames();

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
		if let Some(prog) = ctx.props().progress {
			match prog {
				Progression::Ebook { chapter, page, char_pos } if self.viewing_chapter == 0 => {
					// TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
					self.set_chapter(chapter as usize, ctx);

					if char_pos != -1 {
						let chap = ctx.props().chapters.lock().unwrap();

						let chapter = chap.get(&self.viewing_chapter).unwrap();

						let page = js_get_page_from_byte_position(&chapter.iframe, char_pos as usize);

						if let Some(page) = page {
							chapter.set_page(page);
						}
					}
				}

				_ => ()
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
		if new_total_page == ctx.props().page_count() || self.total_page_position == new_total_page {
			false
		} else {
			let chapter_count = ctx.props().book.chapter_count;
			let chapters = ctx.props().chapters.lock().unwrap();

			let both_pages = self.find_page(self.total_page_position, chapter_count, &*chapters)
				.zip(self.find_page(new_total_page, chapter_count, &*chapters));

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
			let chapters = ctx.props().chapters.lock().unwrap();

			self.total_page_position = 0;
			self.viewing_chapter_page = 0;
			self.viewing_chapter = new_chapter;


			for i in 0..=new_chapter {
				if let Some(chap) = chapters.get(&i) {
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
}



#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
	fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;

	fn js_get_current_byte_pos(iframe: &HtmlIFrameElement) -> Option<usize>;
	fn js_get_page_from_byte_position(iframe: &HtmlIFrameElement, position: usize) -> Option<usize>;
}