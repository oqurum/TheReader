use std::rc::Rc;

use common::api::WrappingResponse;
use common_local::{MemberBasicPreferences, reader::ReaderColor, MemberPreferences};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use crate::{request, components::reader::PageLoadType, AppState};

pub enum Msg {
    // Request Results
    PrefsResult(WrappingResponse<MemberPreferences>),

    // Events
    UpdateSettings(EditingType, Box<dyn Fn(&mut MemberBasicPreferences, serde_json::Value)>, serde_json::Value),

    ContextChanged(Rc<AppState>),

    Submit,
    Ignore,
}

pub struct MemberGeneralPage {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    preferences: MemberPreferences,
}

impl Component for MemberGeneralPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");

        let preferences = state.member.as_ref().unwrap().parse_preferences().unwrap_throw().unwrap_or_default();

        Self {
            state,
            _listener,

            preferences,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => self.state = state,

            Msg::PrefsResult(resp) => match resp.ok() {
                Ok(resp) => self.preferences = resp,
                Err(err) => crate::display_error(err),
            },

            Msg::UpdateSettings(type_of, func, json_value) => {
                match type_of {
                    EditingType::Desktop => func(&mut self.preferences.desktop, json_value),
                    EditingType::Mobile => func(&mut self.preferences.mobile, json_value),
                }

                return false;
            }

            Msg::Submit => {
                let new_prefs = self.preferences.clone();

                ctx.link().send_future(async move {
                    if let Err(e) = request::update_member_preferences(new_prefs).await.ok() {
                        crate::display_error(e);
                        Msg::Ignore
                    } else {
                        crate::request_member_self();

                        Msg::PrefsResult(request::get_member_preferences().await)
                    }
                });
            }

            Msg::Ignore => return false,
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // TODO: Add a check to see which type of device we're on and show settings for said device instead of all devices.

        html! {
            <div class="view-container">
                <div class="col-md-5 col-lg-4">
                    <h2>{ "General Settings" }</h2>

                    <h3>{ "Desktop" }</h3>
                    <hr/>
                    { Self::render_group(EditingType::Desktop, &self.preferences.desktop, ctx) }

                    <h3>{ "Mobile & Tablet" }</h3>
                    <hr/>
                    { Self::render_group(EditingType::Mobile, &self.preferences.mobile, ctx) }

                    // TODO: Possibly something to do with it being "default settings"

                    <button class="btn btn-success" onclick={ ctx.link().callback(|_| Msg::Submit) }>{ "Submit" }</button>
                </div>
            </div>
        }
    }
}

impl MemberGeneralPage {
    fn render_group(editing: EditingType, prefs: &MemberBasicPreferences, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <h4>{ "Reader Settings" }</h4>

                // Auto Full screen?
                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox"
                        checked={ prefs.reader.auto_full_screen }
                        onchange={ ctx.link().callback(move |event: Event| {
                            Msg::UpdateSettings(
                                editing,
                                Box::new(|pref, value| pref.reader.auto_full_screen = value.as_bool().unwrap()),
                                serde_json::Value::Bool(event.target_unchecked_into::<HtmlInputElement>().checked())
                            )
                        }) }
                    />
                    <label class="form-check-label">{ "Fullscreen Reader" }</label>
                </div>

                // Animate Page Transitions?
                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox"
                        checked={ prefs.reader.animate_page_transitions }
                        onchange={ ctx.link().callback(move |event: Event| {
                            Msg::UpdateSettings(
                                editing,
                                Box::new(|pref, value| pref.reader.animate_page_transitions = value.as_bool().unwrap()),
                                serde_json::Value::Bool(event.target_unchecked_into::<HtmlInputElement>().checked())
                            )
                        }) }
                    />
                    <label class="form-check-label">{ "Animate page transitions" }</label>
                </div>

                // Reader Color
                <div class="mb-3">
                    <label class="form-label">{ "Reader Color" }</label>
                    <select class="form-select"
                        onchange={ ctx.link().callback(move |event: Event| {
                            Msg::UpdateSettings(
                                editing,
                                Box::new(|pref, value| pref.reader.color = ReaderColor::from_u8(value.as_u64().unwrap() as u8)),
                                serde_json::Value::Number(event.target_unchecked_into::<HtmlSelectElement>().selected_index().into())
                            )
                        }) }
                    >
                        <option selected={ prefs.reader.color == ReaderColor::Default }>{ "Default" }</option>
                        <option selected={ prefs.reader.color == ReaderColor::Black }>{ "Black" }</option>
                        // TODO: Rest.
                    </select>
                </div>

                // Scale Type
                <div class="mb-3">
                    <label class="form-label">{ "Scale Type" }</label>
                    <select class="form-select" disabled=true>
                        <option>{ "Fit Screen" }</option>
                        <option>{ "Stretch" }</option>
                        <option>{ "Fit Width" }</option>
                        <option>{ "Fit Height" }</option>
                        <option>{ "Original Size" }</option>
                    </select>
                </div>

                // Zoom Landscape Image
                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox" disabled=true />
                    <label class="form-check-label">{ "Zoom Landscape Image" }</label>
                </div>

                // Zoom Start Position
                <div class="mb-3">
                    <label class="form-label">{ "Zoom Start Position" }</label>
                    <select class="form-select" disabled=true>
                        <option>{ "Automatic" }</option>
                        <option>{ "Left" }</option>
                        <option>{ "Right" }</option>
                        <option>{ "Center" }</option>
                    </select>
                </div>

                <div class="mb-3">
                    <label class="form-label">{ "Load Sections By" }</label>
                    <select class="form-select" disabled=true
                        onchange={ ctx.link().callback(move |event: Event| {
                            Msg::UpdateSettings(
                                editing,
                                Box::new(|pref, value| pref.reader.load_type = value.as_u64().unwrap() as u8),
                                serde_json::Value::Number(event.target_unchecked_into::<HtmlSelectElement>().selected_index().into())
                            )
                        }) }
                    >
                        <option selected={ prefs.reader.load_type == u8::from(PageLoadType::All) }>{ "All" }</option>
                        <option selected={ prefs.reader.load_type == u8::from(PageLoadType::Select) }>{ "Select" }</option>
                    </select>
                </div>
            </>
        }
    }
}

#[derive(Clone, Copy)]
pub enum EditingType {
    Desktop,
    Mobile,
}