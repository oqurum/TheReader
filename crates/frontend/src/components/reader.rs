// Move reading.rs into here. Modify reading.rs to utilize this.
// Add dimensions into Property.

// TODO: Batch chapter grabs. Add progress bar at bottom of page.
// TODO: Handle resizing.
// TODO: Define custom size of reader. Currently only fullscreen.

use std::{collections::HashMap, rc::Rc, sync::Mutex};

use books_common::{MediaItem, Chapter};
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}};
use web_sys::{HtmlIFrameElement, HtmlElement};
use yew::prelude::*;

use crate::pages::reading::{ChapterContents, FoundChapterPage};

#[derive(Properties)]
pub struct Property {
	pub book: Rc<MediaItem>,
	pub chapters: Rc<Mutex<HashMap<usize, ChapterContents>>>,

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
	// fn eq(&self, other: &Self) -> bool {
	// 	Rc::ptr_eq(&self.book, &other.book) && Rc::ptr_eq(&self.chapters, &other.chapters) && self.width == other.width && self.height == other.height
	// }

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
}



pub struct GeneratePage {
	container: HtmlElement,
	iframe: HtmlIFrameElement,
	chapter: Chapter,
}


pub struct Reader {
	viewing_page: usize,
	viewing_chapter: usize,

	handle_touch_start: Option<Closure<dyn FnMut(TouchEvent)>>,
	handle_touch_end: Option<Closure<dyn FnMut(TouchEvent)>>,
	handle_touch_cancel: Option<Closure<dyn FnMut(TouchEvent)>>,

	touch_start: Option<(i32, i32)>
}

impl Component for Reader {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			viewing_page: 0,
			viewing_chapter: 0,

			handle_touch_cancel: None,
			handle_touch_end: None,
			handle_touch_start: None,

			touch_start: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::NextPage => {
				if self.viewing_page + 1 == ctx.props().page_count() {
					return false;
				}

				let mut new_chap = None;

				let chapter_count = ctx.props().book.chapter_count;
				let mut chapters = ctx.props().chapters.lock().unwrap();

				// Double check first chapter page count.
				if self.viewing_page == 0 {
					let curr_chap = chapters.get_mut(&0).unwrap();
					curr_chap.page_count = get_iframe_page_count(&curr_chap.iframe).max(1);
				}

				match (
					self.get_page(self.viewing_page, chapter_count, &*chapters),
					self.get_page(self.viewing_page + 1, chapter_count, &*chapters)
				) {
					(Some(c1), Some(c2)) => {
						// Same chapter? Mark it. Don't update renderer.
						if c1 == c2 {
							c2.chapter.set_page(c2.local_page);
						}

						// Different chapter? place back into invisible frame container.
						else {
							log::info!("===== Different chapter");
							self.viewing_chapter = c2.chapter.chapter;
							// c1.chapter.container.append_child(&c1.chapter.iframe).unwrap();
							new_chap = Some(c2.chapter.chapter);
						}
					}

					// Different chapter? place back into invisible frame container.
					(Some(c1), None) => {
						// c1.chapter.container.append_child(&c1.chapter.iframe).unwrap();
					}

					_ => ()
				}

				if let Some(next_chap_number) = new_chap {
					let next_chap = chapters.get_mut(&next_chap_number).unwrap();
					next_chap.page_count = get_iframe_page_count(&next_chap.iframe).max(1);
				}

				self.viewing_page += 1;
			}

			Msg::PreviousPage => {
				if self.viewing_page == 0 {
					return false;
				}

				let mut new_chap = None;

				let chapter_count = ctx.props().book.chapter_count;
				let mut chapters = ctx.props().chapters.lock().unwrap();

				match (
					self.get_page(self.viewing_page, chapter_count, &*chapters),
					self.get_page(self.viewing_page - 1, chapter_count, &*chapters)
				) {
					(Some(c1), Some(c2)) => {
						// Same chapter? Mark it. Don't update renderer.
						if c1 == c2 {
							c2.chapter.set_page(c2.local_page);
						}

						// Different chapter? place back into invisible frame container.
						else {
							self.viewing_chapter = c2.chapter.chapter;
							// c1.chapter.container.append_child(&c1.chapter.iframe).unwrap();
							new_chap = Some(c2.chapter.chapter);
						}
					}

					// Different chapter? place back into invisible frame container.
					(Some(c1), None) => {
						// c1.chapter.container.append_child(&c1.chapter.iframe).unwrap();
					}

					_ => ()
				}

				if let Some(next_chap_number) = new_chap {
					let next_chap = chapters.get_mut(&next_chap_number).unwrap();
					next_chap.page_count = get_iframe_page_count(&next_chap.iframe).max(1);
				}

				self.viewing_page -= 1;
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

		log::info!("VIEW: {}-{}", self.viewing_chapter, self.viewing_page);

		html! {
			<div class="reader">
				<div class="navbar">
					<a onclick={ctx.link().callback(|_| Msg::PreviousPage)}>{"Previous Page"}</a>
					<span>{self.viewing_page + 1} {"/"} {page_count} {" pages"}</span>
					<a onclick={ctx.link().callback(|_| Msg::NextPage)}>{"Next Page"}</a>
				</div>

				<div class="pages" style={pages_style}>
					<div class="frames" style={format!("top: calc(-{}% - {}px);", self.viewing_chapter * 100, self.viewing_chapter as f32 * 3.5)}>
						{ for frames.into_iter().map(|v| Html::VRef(v.into())) }
					</div>
				</div>
			</div>
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		self.update_chapter_pages(ctx);

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
	fn update_chapter_pages(&mut self, ctx: &Context<Self>) {
		let mut chapters = ctx.props().chapters.lock().unwrap();

		for ele in chapters.values_mut() {
			ele.page_count = get_iframe_page_count(&ele.iframe).max(1);
		}
	}

	fn get_page<'a>(&self, find_page: usize, chapter_count: usize, chapters: &'a HashMap<usize, ChapterContents>) -> Option<FoundChapterPage<'a>> {
		log::info!("Get Page {}", find_page);

		let mut curr_page = 0;

		for chap_number in 0..chapter_count {
			let chapter = chapters.get(&chap_number)?;

			let local_page = find_page - curr_page;

			log::info!(
				"    [{}]: r{} < t{}",
				chap_number,
				local_page,
				chapter.page_count// + (chap.page_count % 2)
			);

			if local_page < chapter.page_count {// + (chap.page_count % 2) {
				log::info!("    !!");
				return Some(FoundChapterPage {
					chapter,
					local_page
				});
			}

			curr_page += chapter.page_count;// + (chap.page_count % 2);
		}

		None
	}

	fn view_page(&self, ctx: &Context<Self>) -> Html {
		log::info!("================================");
		log::info!("Viewing Page: {}", self.viewing_page);

		let chapter_count = ctx.props().book.chapter_count;
		let chapters = ctx.props().chapters.lock().unwrap();


		if let Some(found) = self.get_page(self.viewing_page, chapter_count, &*chapters) {
			log::info!("Found Chapter: {}", found.chapter.chapter);

			log::info!("--------------------------------");

			// js_generate_pages_but_horizontally_transpose(&chapter.iframe);

			// if let Some(next_page) = self.get_page(self.viewing_page + 1) {
			// 	let iframe_next = create_iframe();
			// 	iframe_next.set_srcdoc(next_page);

			// 	html! {
			// 		<>
			// 			{ Html::VRef(iframe_curr.into()) }
			// 			{ Html::VRef(iframe_next.into()) }
			// 		</>
			// 	}
			// } else {
				Html::VRef(found.chapter.iframe.clone().into())
			// }
		} else {
			log::info!("--------------------------------");
			html! {}
		}
	}
}



#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
	fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;
	fn js_update_pages_with_inlined_css(iframe: &HtmlIFrameElement);
}