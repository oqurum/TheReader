use yew::prelude::*;

use super::{Popup, PopupType};


// TODO: Implement.
#[derive(Clone, Copy)]
pub enum ButtonPopupPosition {
	Top,
	Bottom,
	Left,
	Right,
}



#[derive(Properties, PartialEq)]
pub struct Property {
	#[prop_or_default]
    pub class: Classes,

	pub children: Children,
}


pub enum Msg {
	TogglePopup,

	ClosePopup,
}


pub struct ButtonPopup {
	is_open: bool,
}

impl Component for ButtonPopup {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			is_open: false,
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::TogglePopup => {
				self.is_open = !self.is_open;
			}

			Msg::ClosePopup => {
				self.is_open = false;
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		html! {
			<div class="button-popup-group">
				<span
					class="button material-icons"
					title="More Options"
					onclick={ctx.link().callback(|_| Msg::TogglePopup)}
				>{ "more_horiz" }</span>

				{
					if self.is_open {
						html! {
							<Popup
								type_of={ PopupType::Display }
								on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
								classes={ classes!("menu-list") }
							>
								{ for ctx.props().children.iter() }
							</Popup>
						}
					} else {
						html! {}
					}
				}
			</div>
		}
	}
}