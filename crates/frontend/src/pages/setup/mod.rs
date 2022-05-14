// TODO: Expand to multiple inlined pages.

use books_common::{api, EditManager, setup::SetupConfig};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::{prelude::*, html::Scope};


pub enum Msg {
	Finish,

	UpdateInput(InputType, String),
}

pub struct SetupPage {
	config: EditManager<SetupConfig>,
}

impl Component for SetupPage {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			config: EditManager::default()
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Finish => {
				//
			}

			Msg::UpdateInput(type_of, value) => match type_of {
				InputType::ServerName => self.config.name = Some(value),
				InputType::Directory => self.config.directories = vec![value],
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		html! {
			<div class="setup-container">
				<div class="main-content-view">
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

impl SetupPage {
	fn render_first(&self, ctx: &Context<Self>) -> Html {
		html! {
			<form>
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
					<button>{ "Continue" }</button>
				</div>
			</form>
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