use std::{rc::Rc, sync::Mutex};

use books_common::api;
use web_sys::HtmlElement;
use yew::{prelude::*, html::Scope};

use crate::request;


static EDITING_CONTAINER_CLASS: &str = "editing-items-inside";

#[derive(Properties)]
pub struct Property {
	pub on_deselect_all: Callback<MouseEvent>,

	pub editing_container: NodeRef,

	pub editing_items: Rc<Mutex<Vec<usize>>>,
}

impl PartialEq for Property {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}


#[derive(Clone)]
pub enum Msg {
	Ignore,
	TogglePopup,

	UpdateMetaByFiles,
}


pub struct MassSelectBar {
	// library_list_ref: NodeRef,
	popup_open: bool,
}

impl Component for MassSelectBar {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			// library_list_ref: NodeRef::default(),
			popup_open: false,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::TogglePopup => {
				self.popup_open = !self.popup_open;
			}

			Msg::UpdateMetaByFiles => {
				self.popup_open = false;

				let meta_ids = {
					let items = ctx.props().editing_items.lock().unwrap();
					items.clone()
				};

				ctx.link()
				.send_future(async move {
					for meta_id in meta_ids {
						request::update_metadata(meta_id, &api::PostMetadataBody::AutoMatchMetaIdByFiles).await;
					}

					Msg::Ignore
				});
			}

			Msg::Ignore => return false,
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		let items = ctx.props().editing_items.lock().unwrap();

		if items.is_empty() {
			html! {}
		} else {
			html! {
				<div class="mass-select-bar">
					<div class="bar-container">
						<div class="left-content">
							<span>{ items.len() } { " items selected" }</span>
						</div>
						<div class="center-content">
							<span
								class="button material-icons"
								title="More Options"
								onclick={ctx.link().callback(|_| Msg::TogglePopup)}
							>{ "more_horiz" }</span>

							{
								if self.popup_open {
									html! {
										<div class="menu-list">
											<div class="menu-item" yew-close-popup="">{ "Refresh Metadata" }</div>
											<div class="menu-item" yew-close-popup="" onclick={
												Self::on_click_prevdef(ctx.link(), Msg::UpdateMetaByFiles)
											}>{ "Quick Search By Files" }</div>
											<div class="menu-item" yew-close-popup="">{ "Delete" }</div>
										</div>
									}
								} else {
									html! {}
								}
							}
						</div>
						<div class="right-content">
							<button onclick={ctx.props().on_deselect_all.clone()}>{ "Deselect All" }</button>
						</div>
					</div>
				</div>
			}
		}
	}

	fn changed(&mut self, ctx: &Context<Self>) -> bool {
		if let Some(container_element) = ctx.props().editing_container.cast::<HtmlElement>() {
			let cl = container_element.class_list();

			if ctx.props().editing_items.lock().unwrap().is_empty() {
				let _ = cl.remove_1(EDITING_CONTAINER_CLASS);
			} else if !cl.contains(EDITING_CONTAINER_CLASS) {
				let _ = cl.add_1(EDITING_CONTAINER_CLASS);
			}
		}


		true
	}

	fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
		//
	}

	fn destroy(&mut self, _ctx: &Context<Self>) {
		//
	}
}

impl MassSelectBar {
	/// A Callback which calls "prevent_default"
	fn on_click_prevdef(scope: &Scope<Self>, msg: Msg) -> Callback<MouseEvent> {
		scope.callback(move |e: MouseEvent| {
			e.prevent_default();
			msg.clone()
		})
	}
}