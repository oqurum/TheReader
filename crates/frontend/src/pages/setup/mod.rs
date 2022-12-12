// TODO: Expand to multiple inlined pages.

use std::path::PathBuf;

use common::{
    api::{ApiErrorResponse, WrappingResponse},
    component::{file_search::FileInfo, FileSearchComponent, FileSearchEvent},
};
use common_local::{
    api::ApiGetSetupResponse,
    setup::{Config, SetupConfig},
    EditManager,
};
use gloo_utils::window;
use validator::{Validate, ValidationErrors};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use super::PasswordLogin;
use crate::{request, BaseRoute};

pub enum SetupPageMessage {
    Ignore,

    AfterSentConfigSuccess,
    AfterSentConfigError(ApiErrorResponse),

    LoginPasswordResponse(std::result::Result<String, ApiErrorResponse>),

    Finish,

    UpdateInput(Box<dyn Fn(&mut EditManager<SetupConfig>, String)>, String),

    IsAlreadySetupResponse(Box<WrappingResponse<ApiGetSetupResponse>>),
}

pub struct SetupPage {
    initial_config: IsSetup,
    config: EditManager<SetupConfig>,
    is_waiting_for_resp: bool,

    current_errors: ValidationErrors,
}

impl Component for SetupPage {
    type Message = SetupPageMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(async move {
            SetupPageMessage::IsAlreadySetupResponse(Box::new(request::check_if_setup().await))
        });

        Self {
            config: EditManager::default(),
            is_waiting_for_resp: false,
            initial_config: IsSetup::Unknown,

            current_errors: ValidationErrors::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            SetupPageMessage::Ignore => return false,

            SetupPageMessage::IsAlreadySetupResponse(resp) => {
                match resp.ok() {
                    Ok(config) => {
                        // TODO: Improve. The only way we know if we've started setup is based off of if the server name is empty or not.
                        self.initial_config = Some(config)
                            .filter(|c| !c.server.name.is_empty())
                            .map(IsSetup::Initially)
                            .unwrap_or(IsSetup::No);

                        if self.is_fully_setup() {
                            // TODO: Add a delay + reason.
                            let history = ctx.link().history().unwrap();
                            history.push(BaseRoute::Dashboard);
                        }
                    }

                    Err(err) => crate::display_error(err),
                }
            }

            SetupPageMessage::AfterSentConfigSuccess => {
                let history = ctx.link().history().unwrap();
                history.push(BaseRoute::Dashboard);
            }

            SetupPageMessage::AfterSentConfigError(err) => {
                self.is_waiting_for_resp = false;
                log::error!("{}", err.description);
                // TODO: Temporary way to show errors.
                crate::display_error(err);
            }

            SetupPageMessage::Finish => {
                if !self.is_waiting_for_resp {
                    self.is_waiting_for_resp = true;

                    let config = self.config.as_changed_value().clone();

                    // Ensure config is valid.
                    if let Err(e) = config.validate() {
                        self.is_waiting_for_resp = false;
                        self.current_errors = e;

                        return true;
                    } else {
                        self.current_errors = ValidationErrors::new();
                    }

                    ctx.link().send_future(async move {
                        match request::finish_setup(config).await.ok() {
                            Ok(_) => SetupPageMessage::AfterSentConfigSuccess,
                            Err(e) => SetupPageMessage::AfterSentConfigError(e),
                        }
                    });
                }

                return false;
            }

            SetupPageMessage::UpdateInput(funky, value) => {
                funky(&mut self.config, value);
            }

            // Admin Creation
            SetupPageMessage::LoginPasswordResponse(resp) => match resp {
                Ok(_) => window().location().reload().unwrap_throw(),
                Err(e) => crate::display_error(e),
            },
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        match &self.initial_config {
            IsSetup::Unknown => html! {
                <div class="setup-container">
                    <div class="view-container">
                        <div class="center-normal">
                            <div class="center-container">
                                <h2>{ "Loading..." }</h2>
                            </div>
                        </div>
                    </div>
                </div>
            },

            IsSetup::Initially(config) => html! {
                <div class={ "view-container setup-view-container" }>
                    <div class="center-normal">
                        <div class="center-container ignore-vertical inner-setup">
                            {
                                if !config.has_admin_account {
                                    self.part_1_account_setup(ctx)
                                } else {
                                    self.part_2_external_authentication(ctx)
                                }
                            }
                        </div>
                    </div>
                </div>
            },

            // TODO: Remove "No" - We will always have a Config File.
            IsSetup::No => html! {
                <div class={ "view-container setup-view-container" }>
                    <div class="center-normal">
                        <div class="center-container ignore-vertical inner-setup">
                            <h2>{ "Setup" }</h2>

                            { self.part_0_render_server_info(ctx) }
                            { self.part_0_render_auth_toggles(ctx) }
                            { self.part_0_render_email_setup(ctx) }

                            <div class="tools">
                                {
                                    if self.current_errors.is_empty() {
                                        html! {}
                                    } else {
                                        html! {
                                            <div class="label red" style="white-space: pre-wrap;">
                                                // TODO: Fix. We should show errors on each input.
                                                // ISSUE: Display: Struct/List aren't properly writeln'd so it will not newline
                                                // https://github.com/Keats/validator/pull/235
                                                { self.current_errors.clone() }
                                            </div>
                                        }
                                    }
                                }

                                <button class="btn btn-primary" disabled={ self.is_waiting_for_resp } onclick={ ctx.link().callback(|_| SetupPageMessage::Finish) }>{ "Continue" }</button>
                            </div>
                        </div>
                    </div>
                </div>
            },
        }
    }
}

impl SetupPage {
    fn is_fully_setup(&self) -> bool {
        match &self.initial_config {
            IsSetup::Unknown => false,
            IsSetup::Initially(config) => config.is_fully_setup(),
            IsSetup::No => false,
        }
    }

    fn part_0_render_server_info(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="navbar-module">
                    <div class="center-content">
                        { "Server Info" }
                    </div>
                </div>

                <div class="mb-3">
                    <label class="form-label" for="our-name">{ "Server Name" }</label>
                    <input
                        class="form-control"
                        id="our-name" type="text"
                        value={ self.config.server.name.clone() }
                        onchange={
                            ctx.link().callback(move |e: Event| SetupPageMessage::UpdateInput(
                                Box::new(|e, v| { e.server.name = v; }),
                                e.target().unwrap().unchecked_into::<HtmlInputElement>().value(),
                            ))
                        }
                    />
                </div>

                <div class="mb-3">
                    <label class="form-label" for="our-directory">{ "Search Directory" }</label>
                    <FileSearchComponent
                        init_location={ PathBuf::from(self.config.directories.first().map(|v| v.as_str()).unwrap_or("/")) }
                        on_event={ ctx.link().callback_future(|e| async move {
                            match e {
                                FileSearchEvent::Request(req) => {
                                    match request::get_directory_contents(req.path.display().to_string()).await.ok() {
                                        Ok(v) => {
                                            req.update.emit((
                                                Some(v.path),
                                                v.items.into_iter()
                                                    .map(|v| FileInfo {
                                                        title: v.title,
                                                        path: v.path,
                                                        is_file: v.is_file,
                                                    })
                                                    .collect()
                                            ));
                                        }

                                        Err(_) => {
                                            req.update.emit((None, Vec::new()));
                                        }
                                    }

                                    SetupPageMessage::Ignore
                                }

                                FileSearchEvent::Submit(directory) => {
                                    SetupPageMessage::UpdateInput(
                                        Box::new(|e, v| { e.directories = vec![v]; }),
                                        directory.display().to_string().replace('\\', "/"),
                                    )
                                }
                            }
                        }) }
                    />
                </div>
            </>
        }
    }

    fn part_0_render_auth_toggles(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="navbar-module">
                    <div class="center-content">
                        { "Authentications" }
                    </div>
                </div>

                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox" id="my-password" checked={ self.config.authenticators.email_pass }
                        onchange={
                            ctx.link().callback(move |_| SetupPageMessage::UpdateInput(
                                Box::new(|e, _| { e.authenticators.email_pass = !e.authenticators.email_pass; }),
                                String::new(),
                            ))
                        }
                    />
                    <label class="form-check-label" for="my-password">{ "Local Password Authentication" }</label>
                </div>

                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox" id="my-passwordless" checked={ self.config.authenticators.email_no_pass }
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
                    <label class="form-check-label" for="my-passwordless">{ "Local Passwordless Authentication" }</label>
                </div>

                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox" id="our-external-auth" disabled=true// checked={ self.config.authenticators.main_server }
                        onchange={
                            ctx.link().callback(move |_| SetupPageMessage::UpdateInput(
                                Box::new(|e, _| { e.authenticators.main_server = !e.authenticators.main_server; }),
                                String::new(),
                            ))
                        }
                    />
                    <label class="form-check-label" for="our-external-auth">{ "External Authentication" }</label>
                </div>
            </>
        }
    }

    fn part_0_render_email_setup(&self, ctx: &Context<Self>) -> Html {
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
                    <div class="mb-3">
                        <label class="form-label" for="display_name">{ "Display Name" }</label>
                        <input
                            class="form-control"
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
                    <div class="mb-3">
                        <label class="form-label" for="sending_email">{ "Email We're Sending From" }</label>
                        <input
                            class="form-control"
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
                    <div class="mb-3">
                        <label class="form-label" for="contact_email">{ "Email We can be contacted by" }</label>
                        <input
                            class="form-control"
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
                    <div class="mb-3">
                        <label class="form-label" for="subject_line">{ "Email Subject Line" }</label>
                        <input
                            class="form-control"
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
                    <div class="mb-3">
                        <label class="form-label" for="smtp_username">{ "SMTP Username" }</label>
                        <input
                            class="form-control"
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
                    <div class="mb-3">
                        <label class="form-label" for="smtp_password">{ "SMTP Password" }</label>
                        <input
                            class="form-control"
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
                    <div class="mb-3">
                        <label class="form-label" for="smtp_relay">{ "SMTP Relay" }</label>
                        <input
                            class="form-control"
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

    // 3rd setup page should be external auth if previously selected.

    fn part_1_account_setup(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <h2>{ "Setup Admin Account" }</h2>

                <PasswordLogin cb={ ctx.link().callback(SetupPageMessage::LoginPasswordResponse) } />
            </>
        }
    }

    fn part_2_external_authentication(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
                <h2>{ "Setup External Authentication" }</h2>

                <span>{ "TODO: Not implemented." }</span>
            </>
        }
    }

    // fn on_change_textarea(scope: &Scope<Self>, updating: ChangingType) -> Callback<Event> {
    //     scope.callback(move |e: Event| {
    //         Msg::UpdateTextArea(updating, e.target().unwrap().dyn_into::<HtmlTextAreaElement>().unwrap().value())
    //     })
    // }
}

enum IsSetup {
    Unknown,
    Initially(Config),
    No,
}
