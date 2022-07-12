// TODO: Expand to multiple inlined pages.

use books_common::{EditManager, setup::SetupConfig};
use validator::{Validate, ValidationErrors};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{request, Route};


pub enum SetupPageMessage {
	AfterSentConfig,

	Finish,

	UpdateInput(Box<dyn Fn(&mut EditManager<SetupConfig>, String)>, String),

	IsAlreadySetupResponse(bool),
}

pub struct SetupPage {
	is_setup: IsSetup,
	config: EditManager<SetupConfig>,
	is_waiting_for_resp: bool,

	current_errors: ValidationErrors,
}

impl Component for SetupPage {
	type Message = SetupPageMessage;
	type Properties = ();

	fn create(ctx: &Context<Self>) -> Self {
		ctx.link().send_future(async move {
			SetupPageMessage::IsAlreadySetupResponse(request::check_if_setup().await)
		});

		Self {
			config: EditManager::default(),
			is_waiting_for_resp: false,
			is_setup: IsSetup::Unknown,

			current_errors: ValidationErrors::new(),
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			SetupPageMessage::IsAlreadySetupResponse(is_setup) => {
				self.is_setup = if is_setup {
					// TODO: Add a delay.
					let history = ctx.link().history().unwrap();
    				history.push(Route::Dashboard);

					IsSetup::Yes
				} else {
					IsSetup::No
				};
			}

			SetupPageMessage::AfterSentConfig => {
				let history = ctx.link().history().unwrap();
				history.push(Route::Dashboard);
			}

			SetupPageMessage::Finish => {
				if !self.is_waiting_for_resp {
					self.is_waiting_for_resp = true;

					let config = self.config.as_changed_value().clone();

					// TODO: Add Response to request::finish_setup

					// Ensure config is valid.
					if let Err(e) = config.validate() {
						self.current_errors = e;

						return true;
					} else {
						self.current_errors = ValidationErrors::new();
					}

					ctx.link().send_future(async move {
						let is_okay = request::finish_setup(config).await;

						if is_okay {
							log::info!("Successfully setup.");
						}

						SetupPageMessage::AfterSentConfig
					});
				}

				return false;
			}

			SetupPageMessage::UpdateInput(funky, value) => {
				funky(&mut self.config, value);
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
								<span>{ "Redirecting..." }</span>
							</div>
						</div>
					</div>
				</div>
			},

			IsSetup::No => html! {
				<div class={ "main-content-view setup-view-container" }>
					<div class="center-normal">
						<div class="center-container ignore-vertical">
							<h2>{ "Setup" }</h2>

							{ self.render_server_info(ctx) }
							{ self.render_auth_toggles(ctx) }
							{ self.render_email_setup(ctx) }

							<div>
								{
									if self.current_errors.is_empty() {
										html! {}
									} else {
										html! {
											<div class="label red" style="white-space: pre-wrap;">
												// TODO: Fix. We should show errors on each input.
												// ISSUE: Display: Struct/List aren't properly writeln'd so it will not newline
												// https://github.com/Keats/validator/blob/master/validator/src/display_impl.rs#L49
												{ self.current_errors.clone() }
											</div>
										}
									}
								}

								<button disabled={ self.is_waiting_for_resp } onclick={ ctx.link().callback(|_| SetupPageMessage::Finish) }>{ "Continue" }</button>
							</div>
						</div>
					</div>
				</div>
			}
		}
	}
}

impl SetupPage {
	fn render_server_info(&self, ctx: &Context<Self>) -> Html {
		html! {
			<>
				<div class="navbar-module">
					<div class="center-content">
						{ "Server Info" }
					</div>
				</div>
				<div class="form-container">
					<label for="our-name">{ "Server Name" }</label>
					<input
						id="our-name" type="text"
						value={ self.config.server_name.clone() }
						onchange={
							ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
								Box::new(|e, v| { e.server_name = v; }),
								e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
							))
						}
					/>
				</div>

				<div class="form-container">
					<label for="our-directory">{ "What directory?" }</label>
					<input
						id="our-directory" type="text"
						value={ self.config.directories.first().map(|v| v.to_string()) }
						onchange={
							ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
								Box::new(|e, v| { e.directories = vec![v]; }),
								e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
							))
						}
					/>
				</div>
			</>
		}
	}

	fn render_auth_toggles(&self, ctx: &Context<Self>) -> Html {
		html! {
			<>
				<div class="navbar-module">
					<div class="center-content">
						{ "Authentications" }
					</div>
				</div>
				<div class="form-container">
					<div class="row">
						<input type="checkbox" id="my-password" checked={ self.config.authenticators.email_pass }
							onchange={
								ctx.link().callback(move |_| SetupPageMessage::UpdateInput(
									Box::new(|e, _| { e.authenticators.email_pass = !e.authenticators.email_pass; }),
									String::new(),
								))
							}
						/>
						<label for="my-password">{ "Local Password Authentication" }</label>
					</div>

					<div class="row">
						<input type="checkbox" id="my-passwordless" checked={ self.config.authenticators.email_no_pass }
							onchange={
								ctx.link().callback(move |_| SetupPageMessage::UpdateInput(
									Box::new(|e, _| {
										e.authenticators.email_no_pass = !e.authenticators.email_no_pass;

										if e.authenticators.email_no_pass {
											e.email = Some(Default::default());
										} else {
											e.email = None;
										}
									}),
									String::new(),
								))
							}
						/>
						<label for="my-passwordless">{ "Local Passwordless Authentication" }</label>
					</div>

					<div class="row">
						<input type="checkbox" id="our-external-auth" checked={ self.config.authenticators.main_server }
							onchange={
								ctx.link().callback(move |_| SetupPageMessage::UpdateInput(
									Box::new(|e, _| { e.authenticators.main_server = !e.authenticators.main_server; }),
									String::new(),
								))
							}
						/>
						<label for="our-external-auth">{ "External Authentication" }</label>
					</div>
				</div>
			</>
		}
	}

	fn render_email_setup(&self, ctx: &Context<Self>) -> Html {
		if self.config.authenticators.email_no_pass {
			let email = self.config.email.clone().unwrap_or_default();

			html! {
				<>
					<div class="navbar-module">
						<div class="center-content">
							{ "Passwordless Email Setup" }
						</div>
					</div>

					<div class="label yellow">{ "Must fill out ALL these fields to use the Passwordless Login." }</div>

					// Display Name
					<div class="form-container">
						<label for="display_name">{ "Display Name" }</label>
						<input
							id="display_name" type="text"
							placeholder="The Ultimate Book Reading Library"
							value={ email.display_name }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().display_name = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>

					// Sending Emails From
					<div class="form-container">
						<label for="sending_email">{ "Email We're Sending From" }</label>
						<input
							id="sending_email" type="text"
							placeholder="from@example.com"
							value={ email.sending_email }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().sending_email = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>

					// Contact Email
					<div class="form-container">
						<label for="contact_email">{ "Email We can be contacted by" }</label>
						<input
							id="contact_email" type="text"
							placeholder="contact@example.com"
							value={ email.contact_email }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().contact_email = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>

					// Email Subject Line
					<div class="form-container">
						<label for="subject_line">{ "Email Subject Line" }</label>
						<input
							id="subject_line" type="text"
							placeholder="Your link to sign in to The Ultimate Library"
							value={ email.subject_line }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().subject_line = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>

					// SMTP Username
					<div class="form-container">
						<label for="smtp_username">{ "SMTP Username" }</label>
						<input
							id="smtp_username" type="text"
							placeholder="(can be found on your email provider)"
							value={ email.smtp_username }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().smtp_username = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>

					// SMTP Password
					<div class="form-container">
						<label for="smtp_password">{ "SMTP Password" }</label>
						<input
							id="smtp_password" type="text"
							placeholder="(can be found on your email provider)"
							value={ email.smtp_password }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().smtp_password = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>

					// SMTP Relay
					<div class="form-container">
						<label for="smtp_relay">{ "SMTP Relay" }</label>
						<input
							id="smtp_relay" type="text"
							placeholder="(can be found on your email provider)"
							value={ email.smtp_relay }
							onchange={
								ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
									Box::new(|e, v| { e.get_email_mut().smtp_relay = v; }),
									e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
								))
							}
						/>
					</div>
				</>
			}
		} else {
			html! {}
		}

	}

	// fn on_change_textarea(scope: &Scope<Self>, updating: ChangingType) -> Callback<Event> {
	// 	scope.callback(move |e: Event| {
	// 		Msg::UpdateTextArea(updating, e.target().unwrap().dyn_into::<HtmlTextAreaElement>().unwrap().value())
	// 	})
	// }
}


#[derive(Clone, Copy, PartialEq)]
enum IsSetup {
	Unknown,
	Yes,
	No
}