use wasm_bindgen::prelude::wasm_bindgen;
use yew::prelude::*;

#[derive(Properties)]
pub struct Property {
	pub visible: bool
}

impl PartialEq for Property {
	fn eq(&self, other: &Self) -> bool {
		self.visible == other.visible
	}
}


pub enum Msg {
	//
}


pub struct Notes {
	is_initiated: bool
}

impl Component for Notes {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			is_initiated: false
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
		false
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if ctx.props().visible {
			html! {
				<div class="notes">
					<div id="notary"></div>
				</div>
			}
		} else {
			html! {}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
		if !self.is_initiated && ctx.props().visible {
			registerNotesApp();
			self.is_initiated = true;
		}
	}

	fn changed(&mut self, _ctx: &Context<Self>) -> bool {
		self.is_initiated = false;

		true
	}
}

impl Notes {
	//
}


#[wasm_bindgen(module = "/notes.js")]
extern "C" {
	fn registerNotesApp();
}