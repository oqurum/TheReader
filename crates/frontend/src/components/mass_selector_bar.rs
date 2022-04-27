use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Property {
	//
}


pub enum Msg {
	//
}


pub struct MassSelectBar {
	//
}

impl Component for MassSelectBar {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			//
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
		false
	}

	fn view(&self, _ctx: &Context<Self>) -> Html {
		html! {
			<div class="mass-select-bar">
				<div class="bar-container">
					<div class="left-content">
						<span>{ "1 items selected" }</span>
					</div>
					<div class="center-content">
						<span class="button material-icons" title="More Options">{ "more_horiz" }</span>
					</div>
					<div class="right-content">
						<span>{ "Deselect All" }</span>
					</div>
				</div>
			</div>
		}
	}

	fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
		//
	}

	fn destroy(&mut self, _ctx: &Context<Self>) {
		//
	}
}