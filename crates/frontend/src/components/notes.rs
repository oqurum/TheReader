use std::rc::Rc;

use books_common::MediaItem;
use gloo_timers::callback::Timeout;
use serde_json::json;
use wasm_bindgen::{prelude::{wasm_bindgen, Closure}, JsValue, JsCast};
use yew::prelude::*;

use crate::request;

#[derive(Properties)]
pub struct Property {
	pub book: Rc<MediaItem>
}

impl PartialEq for Property {
	fn eq(&self, other: &Self) -> bool {
		self.book == other.book
	}
}


pub enum Msg {
	RetrieveNotes(Option<String>),

	OnDeltaChanged(JsValue),
	OnAutoSave,

	Ignore
}


pub struct Notes {
	is_initiated: bool,
	quill: Option<QuillContents>,
	contents: JsValue,
	timeout: Option<Timeout>
}

impl Component for Notes {
	type Message = Msg;
	type Properties = Property;

	fn create(ctx: &Context<Self>) -> Self {
		let book_id = ctx.props().book.id;

		ctx.link()
		.send_future(async move {
			Msg::RetrieveNotes(request::get_book_notes(book_id).await)
		});

		Self {
			is_initiated: false,
			contents: JsValue::from_serde(&json!([])).unwrap(),
			quill: None,
			timeout: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Ignore => (),

			Msg::OnAutoSave => {
				log::info!("Auto Save");

				self.timeout = None;

				if let Some(notes) = self.quill.as_ref() {
					let book_id = ctx.props().book.id;
					let body = js_sys::JSON::stringify(&notes.quill.get_contents()).unwrap().as_string().unwrap();

					ctx.link()
					.send_future(async move {
						request::update_book_notes(book_id, body).await;

						Msg::Ignore
					});
				}
			}

			Msg::OnDeltaChanged(_delta_changes) => {
				let scope = ctx.link().clone();
				self.timeout = Some(Timeout::new(
					3_000,
					move || scope.send_message(Msg::OnAutoSave)
				));
			}

			Msg::RetrieveNotes(data) => {
				if let Some(data) = data.filter(|v| !v.is_empty()) {
					self.contents = js_sys::JSON::parse(&data).unwrap();

					if let Some(quill) = self.quill.as_ref() {
						quill.quill.set_contents(&self.contents, None);
					}
				}
			}
		}

		false
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		html! {
			<div class="notes">
				<div id="notary"></div>
			</div>
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
		if !self.is_initiated {
			let quill = Quill::new(
				"#notary",
				&JsValue::from_serde(&serde_json::json!({
					"modules": {
						"toolbar": [
							[ "bold", "italic", "underline", "strike" ],
							[ "blockquote", "code-block" ],

							[ { "header": 1 }, { "header": 2 } ],
							[ { "list": "ordered" }, { "list": "bullet" } ],
							[ { "script": "sub" }, { "script": "super" } ],
							[ { "indent": "-1" }, { "indent": "+1" } ],
							[ { "direction": "rtl" } ],

							[ { "size": [ "small", false, "large", "huge" ] } ],
							[ { "header": [ 1, 2, 3, 4, 5, 6, false ] } ],

							[ { "color": [] }, { "background": [] } ],
							[ { "font": [] } ],
							[ { "align": [] } ],

							// [
							// 	"formula",
							// 	"video",
							// 	"image"
							// ],

							["clean"]
						]
					},
					"placeholder": "Compose an epic...",
					"theme": "snow"
				})).unwrap()
			);

			let scope = ctx.link().clone();
			let closure = Closure::wrap(
				Box::new(move |delta: JsValue| scope.send_message(Msg::OnDeltaChanged(delta))) as Box<dyn FnMut(JsValue)>
			);

			quill.on("text-change", closure.as_ref().unchecked_ref());
			quill.set_contents(&self.contents, None);

			self.quill = Some(QuillContents {
				quill,
				closure
			});

			self.is_initiated = true;
		}
	}

	fn changed(&mut self, _ctx: &Context<Self>) -> bool {
		self.is_initiated = false;
		self.quill = None;

		true
	}
}

impl Notes {
	//
}


struct QuillContents {
	quill: Quill,
	#[allow(dead_code)]
	closure: Closure<dyn FnMut(JsValue)>
}

#[wasm_bindgen]
extern "C" {
	#[derive(Debug)]
	type Quill;

	#[wasm_bindgen(constructor)]
    fn new(id: &str, options: &JsValue) -> Quill;

	#[wasm_bindgen(method)]
	fn import(this: &Quill, value: &str) -> JsValue;

	#[wasm_bindgen(method)]
	fn on(this: &Quill, value: &str, closure: &js_sys::Function);

	#[wasm_bindgen(method, js_name = getContents)]
	fn get_contents(this: &Quill) -> JsValue;

	#[wasm_bindgen(method, js_name = setContents)]
	fn set_contents(this: &Quill, value: &JsValue, source: Option<&str>) -> JsValue;
}

// TODO: Check for [page:10]

// TODO: Add page button markers
// const icons = Quill.import('ui/icons');
// const Inline = Quill.import('blots/inline');
// const Delta = Quill.import('delta');

// class Page extends Inline {
// 	static create(value) {
// 		console.log('create');
// 		let node = super.create(value);
// 		value = this.sanitize(value);
// 		node.setAttribute('href', value);
// 		node.setAttribute('target', '_blank');
// 		return node;
// 	}

// 	static formats(domNode) {
// 		return domNode.getAttribute('href');
// 	}

// 	static sanitize(url) {
// 		let anchor = document.createElement('a');
// 		anchor.href = url;
// 		let protocol = anchor.href.slice(0, anchor.href.indexOf(':'));
// 		return this.PROTOCOL_WHITELIST.indexOf(protocol) > -1;
// 	}

// 	format(name, value) {
// 		if (name !== this.statics.blotName || !value) return super.format(name, value);
// 		value = this.constructor.sanitize(value);
// 		this.domNode.setAttribute('href', value);
// 	}
// }

// Page.blotName = 'page';
// Page.tagName = 'A';
// Page.SANITIZED_URL = 'about:blank';
// Page.PROTOCOL_WHITELIST = ['http', 'https', 'mailto', 'tel'];

// Quill.register('formats/page', Page);

// icons.page = icons.bold;