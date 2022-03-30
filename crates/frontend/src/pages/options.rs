use books_common::{api, BasicLibrary, BasicDirectory};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement};
use yew::prelude::*;

use crate::request;

pub enum Msg {
	// Request Results
	OptionsResults(api::GetOptionsResponse),

	// Events
	DisplayPopup(usize, i64),
	ClosePopup,

	RequestUpdateOptions(bool),
	UpdatePopup(api::ModifyOptionsBody),

	Ignore
}

pub struct OptionsPage {
	resp: Option<api::GetOptionsResponse>,
	visible_popup: Option<(usize, i64)>,
	update_popup: Option<api::ModifyOptionsBody>
}

impl Component for OptionsPage {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			resp: None,
			visible_popup: None,
			update_popup: None
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::OptionsResults(resp) => {
				self.resp = Some(resp);
				self.visible_popup = None;
			}

			Msg::DisplayPopup(popup, index) => {
				self.visible_popup = Some((popup, index));
			}

			Msg::ClosePopup => {
				self.visible_popup = None;
			}

			Msg::UpdatePopup(update) => {
				self.update_popup = Some(update);
			}

			Msg::RequestUpdateOptions(should_add_options) => {
				if let Some(options) = self.update_popup.take() {
					if should_add_options {
						ctx.link().send_future(async {
							request::update_options_add(options).await;
							Msg::OptionsResults(request::get_options().await)
						});
					} else {
						ctx.link().send_future(async {
							request::update_options_remove(options).await;
							Msg::OptionsResults(request::get_options().await)
						});
					}
				}
			}

			Msg::Ignore => ()
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(resp) = self.resp.as_ref() {
			html! {
				<div class="options-page">
					<h2>{ "Tasks" }</h2>

					<button class="button" onclick={ ctx.link().callback_future(|_| async {
						request::run_task().await;
						Msg::Ignore
					}) }>{ "Run Library Scan + Metadata Updater" }</button>


					<h2>{ "Libraries" }</h2>
					{
						for resp.libraries.iter()
							.map(|v| {
								let lib_id = v.id;

								html! {
									<>
										<h3>{ v.name.clone() }</h3>
										<button class="button" onclick={ ctx.link().batch_callback(move|_| {
											vec![
												Msg::UpdatePopup(api::ModifyOptionsBody {
													library: Some(BasicLibrary {
														id: Some(lib_id),
														name: None
													}),
													directory: None
												}),
												Msg::RequestUpdateOptions(false)
											]
										}) }>{ "delete" }</button>
										<ul>
											{
												for v.directories.iter().map(move |v| {
													let path = v.clone();

													html! {
														<li><button class="button" onclick={ ctx.link().batch_callback(move |_| {
															vec![
																Msg::UpdatePopup(api::ModifyOptionsBody {
																	library: None,
																	directory: Some(BasicDirectory {
																		library_id: lib_id,
																		path: path.clone()
																	})
																}),
																Msg::RequestUpdateOptions(false)
															]
														}) }>{ "X" }</button>{ v.clone() }</li>
													}
												})
											}
											<li><button class="button" onclick={ctx.link().callback(move |_| Msg::DisplayPopup(1, lib_id))}>{ "Add New" }</button></li>
										</ul>
									</>
								}
							})
					}
					<button class="button" onclick={ctx.link().callback(|_| Msg::DisplayPopup(0, 0))}>{ "Add Library" }</button>

					{ self.render_popup(ctx) }
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
			ctx.link()
			.send_future(async {
				Msg::OptionsResults(request::get_options().await)
			});
		}
	}
}

impl OptionsPage {
	fn render_popup(&self, ctx: &Context<Self>) -> Html {
		if let Some((popup_id, item_index)) = self.visible_popup {
			// TODO: Make popup component for this.

			match popup_id {
				// Add Library
				0 => html! {
					<div class="popup" onclick={ctx.link().callback(|e: MouseEvent| {
						if e.target().map(|v| v.dyn_into::<HtmlElement>().unwrap().class_list().contains("popup")).unwrap_or_default() { Msg::ClosePopup } else { Msg::Ignore }
					})}>
						<div style="background-color: #2b2c37; padding: 10px; border-radius: 3px;">
							<h2>{ "New Library" }</h2>

							<input type="text" name="name" placeholder="Library Name" value="" onchange={ ctx.link().callback(|e: Event| {
								Msg::UpdatePopup(api::ModifyOptionsBody {
									library: Some(BasicLibrary {
										id: None,
										name: Some(e.target_unchecked_into::<HtmlInputElement>().value())
									}),
									directory: None
								})
							}) } />

							<button class="button" onclick={ ctx.link().callback(|_| Msg::RequestUpdateOptions(true)) }>{"Create"}</button>
						</div>
					</div>
				},

				// Add Directory to Library
				1 => html! {
					<div class="popup" onclick={ctx.link().callback(|e: MouseEvent| {
						if e.target().map(|v| v.dyn_into::<HtmlElement>().unwrap().class_list().contains("popup")).unwrap_or_default() { Msg::ClosePopup } else { Msg::Ignore }
					})}>
						<div style="background-color: #2b2c37; padding: 10px; border-radius: 3px;">
							<h2>{ "Add Directory to Library" }</h2>

							// TODO: Directory Selector
							<input type="text" name="directory" placeholder="Directory" value="" onchange={ ctx.link().callback(move |e: Event| {
								Msg::UpdatePopup(api::ModifyOptionsBody {
									library: None,
									directory: Some(BasicDirectory {
										library_id: item_index,
										path: e.target_unchecked_into::<HtmlInputElement>().value(),
									})
								})
							}) } />

							<button class="button" onclick={ ctx.link().callback(|_| Msg::RequestUpdateOptions(true)) }>{"Create"}</button>
						</div>
					</div>
				},

				_ => html! {}
			}

		} else {
			html! {}
		}
	}
}