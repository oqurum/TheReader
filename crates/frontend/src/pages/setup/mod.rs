// TODO: Expand to multiple inlined pages.

use books_common::{api, EditManager, setup::SetupConfig};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::{prelude::*, html::Scope};
use yew_router::prelude::*;

use crate::{request, Route};


pub enum Msg {
	AfterSentConfig,

	Finish,

	UpdateInput(InputType, String),

	IsAlreadySetupResponse(bool),
}

pub struct SetupPage {
	is_setup: IsSetup,
	config: EditManager<SetupConfig>,
	is_finishing: bool,
}

impl Component for SetupPage {
	type Message = Msg;
	type Properties = ();

	fn create(ctx: &Context<Self>) -> Self {
		ctx.link().send_future(async move {
			Msg::IsAlreadySetupResponse(request::check_if_setup().await)
		});

		Self {
			config: EditManager::default(),
			is_finishing: false,
			is_setup: IsSetup::Unknown,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::IsAlreadySetupResponse(is_setup) => {
				self.is_setup = if is_setup {
					let history = ctx.link().history().unwrap();
    				history.push(Route::Dashboard);

					IsSetup::Yes
				} else {
					IsSetup::No
				};
			}

			Msg::AfterSentConfig => {
				let history = ctx.link().history().unwrap();
				history.push(Route::Dashboard);
			}

			Msg::Finish => {
				if !self.is_finishing {
					self.is_finishing = true;

					let config = self.config.as_changed_value().clone();

					ctx.link().send_future(async move {
						let is_okay = request::finish_setup(config).await;

						if is_okay {
							log::info!("Successfully setup.");
						}

						Msg::AfterSentConfig
					});
				}

				return false;
			}

			Msg::UpdateInput(type_of, value) => match type_of {
				InputType::ServerName => self.config.name = Some(value),
				InputType::Directory => self.config.directories = vec![value],
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		match self.is_setup {
			IsSetup::Unknown => html! {
				<div class="setup-container">
					<div class="main-content-view">
						<div class="center-normal">
							<div class="center-container">
								<h2>{ "Loading..." }</h2>
							</div>
						</div>
					</div>
				</div>
			},

			IsSetup::Yes => html! {
				<div class="setup-container">
					<div class="main-content-view">
						<div class="center-normal">
							<div class="center-container">
								<h2>{ "Already Setup..." }</h2>
							</div>
						</div>
					</div>
				</div>
			},

			IsSetup::No => html! {
				<div class="setup-container">
					<div class="main-content-view">
						<h2>{ "Setup" }</h2>
						<div class="center-normal">
							<div class="center-container">
								{ self.render_first(ctx) }
							</div>
						</div>
					</div>
				</div>
			}
		}
	}
}

impl SetupPage {
	fn render_first(&self, ctx: &Context<Self>) -> Html {
		html! {
			<div>
				<div class="form-container">
					<label for="our-name">{ "What would you like to name me?" }</label>
					<input
						id="our-name" type="text"
						value={self.config.name.clone()}
						onchange={Self::on_change_input(ctx.link(), InputType::ServerName)}
					/>
				</div>
				<div class="form-container">
					<label for="our-directory">{ "What directory?" }</label>
					<input
						id="our-directory" type="text"
						value={self.config.directories.first().map(|v| v.to_string())}
						onchange={Self::on_change_input(ctx.link(), InputType::Directory)}
					/>
				</div>

				<div>
					<button disabled={self.is_finishing} onclick={ctx.link().callback(|_| Msg::Finish)}>{ "Continue" }</button>
				</div>
			</div>
		}
	}


	fn on_change_input(scope: &Scope<Self>, updating: InputType) -> Callback<Event> {
		scope.callback(move |e: Event| {
			Msg::UpdateInput(updating, e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value())
		})
	}

	// fn on_change_textarea(scope: &Scope<Self>, updating: ChangingType) -> Callback<Event> {
	// 	scope.callback(move |e: Event| {
	// 		Msg::UpdateTextArea(updating, e.target().unwrap().dyn_into::<HtmlTextAreaElement>().unwrap().value())
	// 	})
	// }
}


#[derive(Clone, Copy)]
pub enum InputType {
	ServerName,
	Directory,
}



#[derive(Clone, Copy, PartialEq)]
enum IsSetup {
	Unknown,
	Yes,
	No
}