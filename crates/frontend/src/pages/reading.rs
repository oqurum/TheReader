// TODO: Handle resizing.
// TODO: Allow custom sizes.

use std::{rc::Rc, sync::Mutex};

use books_common::{MediaItem, api::{GetBookIdResponse, GetChaptersResponse}, Progression};
use yew::prelude::*;

use crate::{request, components::reader::LoadedChapters};
use crate::components::reader::Reader;
use crate::components::notes::Notes;


pub enum Msg {
	// Event
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
	chapters: Rc<Mutex<LoadedChapters>>,
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
			chapters: Rc::new(Mutex::new(LoadedChapters::new())),
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

			Msg::RetrievePages(mut info) => {
				let mut chap_container = self.chapters.lock().unwrap();

				self.last_grabbed_count = info.limit;
				chap_container.total = info.total;

				chap_container.chapters.append(&mut info.chapters);
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
							progress={self.progress}
							book={Rc::clone(book)}
							chapters={Rc::clone(&self.chapters)}
							width={width}
							height={height}
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