use yew::prelude::*;

#[derive(Properties)]
pub struct Property {
	pub visible: bool
}

impl PartialEq for Property {
	fn eq(&self, _other: &Self) -> bool {
		false
	}
}


pub enum Msg {
	//
}


pub struct Notes {
}

impl Component for Notes {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if ctx.props().visible {
			html! {
				<div class="notes">
					<h1>{ "Notes" }</h1>
				</div>
			}
		} else {
			html! {}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		//
	}
}

impl Notes {
	//
}