use std::sync::{Arc, Mutex};

use gloo_utils::body;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{MouseEvent, Element};
use yew::prelude::*;


pub mod edit_metadata;


#[derive(Clone, Copy, PartialEq)]
pub enum PopupType {
	/// Full foreground overlay
	FullOverlay,
	/// Places the popover at the specified point and attempts to keep it there while staying readable.
	AtPoint(i32, i32)
}

impl PopupType {
	pub fn should_exit(self, element: Element) -> bool {
		match self {
			// If we clicked .popup
			Self::FullOverlay if element.class_list().contains("popup") => true,
			// If we didn't click inside of the container
			Self::AtPoint(_, _) if !does_parent_contain_class(&element, "popup-at-point") => true,
			// Otherwise just check for a "data-close-popup" attribute
			_ => does_parent_contain_attribute(&element, "yew-close-popup")
		}
	}
}


#[derive(Properties, PartialEq)]
pub struct Property {
	#[prop_or_default]
    pub classes: Classes,

	pub children: Children,
	pub type_of: PopupType,

	pub on_close: Callback<()>
}


pub enum Msg {
	//
}


pub struct Popup {
	node_ref: NodeRef,
	#[allow(clippy::type_complexity)]
	closure: Arc<Mutex<Option<Closure<dyn FnMut(MouseEvent)>>>>,
}

impl Component for Popup {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			node_ref: NodeRef::default(),
			closure: Arc::new(Mutex::new(None)),
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
		false
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		match ctx.props().type_of {
			PopupType::FullOverlay => html! {
				<div ref={self.node_ref.clone()} class="popup">
					<div class={classes!("popup-container", ctx.props().classes.clone())}>
						{ for ctx.props().children.iter() }
					</div>
				</div>
			},

			PopupType::AtPoint(pos_x, pos_y) => {
				let styling = format!("left: {}px; top: {}px;", pos_x, pos_y);

				html! {
					<div ref={self.node_ref.clone()} class={classes!("popup-at-point", ctx.props().classes.clone())} style={ styling }>
						{ for ctx.props().children.iter() }
					</div>
				}
			}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
		// TODO: On render check dimensions of and adjust "AtPoint"

		// FIX: rendered would be called again if we clicked an element containing another onclick event.
		// Resulted in our previous event being overwritten but not removed from the listener.
		if let Some(func) = (*self.closure.lock().unwrap()).take() {
			let _ = body().remove_event_listener_with_callback("click", func.as_ref().unchecked_ref());
		}

		let closure = Arc::new(Mutex::default());
		let c2 = closure.clone();

		let viewing = ctx.props().type_of;
		let exit_fn = ctx.props().on_close.clone();

		let on_click = Closure::wrap(Box::new(move |event: MouseEvent| {
			let _test = c2.clone();

			if let Some(target) = event.target() {
				if viewing.should_exit(target.unchecked_into()) {
					exit_fn.emit(());
				}
			}
		}) as Box<dyn FnMut(MouseEvent)>);

		let _ = body().add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());

		*closure.lock().unwrap() = Some(on_click);

		self.closure = closure;
	}

	fn destroy(&mut self, _ctx: &Context<Self>) {
		let func = (*self.closure.lock().unwrap()).take().unwrap();
		let _ = body().remove_event_listener_with_callback("click", func.as_ref().unchecked_ref());
	}
}

fn does_parent_contain_class(element: &Element, value: &str) -> bool {
	if element.class_list().contains(value) {
		true
	} else if let Some(element) = element.parent_element() {
		does_parent_contain_class(&element, value)
	} else {
		false
	}
}

fn does_parent_contain_attribute(element: &Element, value: &str) -> bool {
	if element.has_attribute(value) {
		true
	} else if let Some(element) = element.parent_element() {
		does_parent_contain_attribute(&element, value)
	} else {
		false
	}
}