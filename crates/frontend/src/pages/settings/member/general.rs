use std::rc::Rc;

use common::component::select::{SelectItem, SelectModule};
use common_local::{
    reader::{LayoutType, ReaderColor, ReaderLoadType},
    GeneralBookPreferences, MemberPreferences, ReaderImagePreferences,
};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{get_preferences, save_preferences, util::is_mobile_or_tablet, AppState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabVisible {
    TextReader,
    ImageReader,
}

pub enum Msg {
    // Events
    UpdateSettingsGeneral(
        EditingType,
        Box<dyn Fn(&mut GeneralBookPreferences, serde_json::Value)>,
        serde_json::Value,
    ),

    ContextChanged(Rc<AppState>),

    ChangeTab(TabVisible),
    ToggleGrouping(usize),

    Submit,
}

pub struct MemberGeneralPage {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    preferences: MemberPreferences,

    viewing_tab: TabVisible,
    is_desktop: bool,

    visible_groupings: [bool; 4],
}

impl Component for MemberGeneralPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");

        let preferences: MemberPreferences = get_preferences().unwrap_throw().unwrap_or_default();

        let is_desktop = !is_mobile_or_tablet();

        Self {
            state,
            _listener,

            preferences,

            viewing_tab: TabVisible::TextReader,
            is_desktop,

            visible_groupings: if is_desktop {
                [true, false, true, false]
            } else {
                [false, true, false, true]
            },
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => self.state = state,

            Msg::ChangeTab(tab) => self.viewing_tab = tab,

            Msg::ToggleGrouping(group) => match self.viewing_tab {
                TabVisible::TextReader => {
                    self.visible_groupings[group] = !self.visible_groupings[group]
                }
                TabVisible::ImageReader => {
                    self.visible_groupings[group + 2] = !self.visible_groupings[group + 2]
                }
            },

            Msg::UpdateSettingsGeneral(type_of, func, json_value) => {
                match (self.viewing_tab, type_of) {
                    (TabVisible::TextReader, EditingType::Desktop) => {
                        func(&mut self.preferences.text_book.desktop.general, json_value)
                    }
                    (TabVisible::TextReader, EditingType::Mobile) => {
                        func(&mut self.preferences.text_book.mobile.general, json_value)
                    }
                    (TabVisible::ImageReader, EditingType::Desktop) => {
                        func(&mut self.preferences.image_book.desktop.general, json_value)
                    }
                    (TabVisible::ImageReader, EditingType::Mobile) => {
                        func(&mut self.preferences.image_book.mobile.general, json_value)
                    }
                }

                return false;
            }

            Msg::Submit => {
                if let Err(e) = save_preferences(&self.preferences) {
                    crate::display_error(e.into());
                }
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // TODO: Add a check to see which type of device we're on and show settings for said device instead of all devices.

        html! {
            <div class="view-container">
                <div class="col-md-5 col-lg-4">
                    <h2>{ "General Settings" }</h2>

                    <nav class="nav nav-pills">
                        <a
                            onclick={ ctx.link().callback(|_| Msg::ChangeTab(TabVisible::TextReader)) }
                            class={ classes!("nav-link", (self.viewing_tab == TabVisible::TextReader).then_some("active")) }
                            href="javascript:{}"
                        >{ "Text Reader" }</a>
                        <a
                            onclick={ ctx.link().callback(|_| Msg::ChangeTab(TabVisible::ImageReader)) }
                            class={ classes!("nav-link", (self.viewing_tab == TabVisible::ImageReader).then_some("active")) }
                            href="javascript:{}"
                        >{ "Image Reader" }</a>
                    </nav>

                    {
                        match self.viewing_tab {
                            TabVisible::TextReader => self.render_text_reader_tab(&self.visible_groupings[0..2], ctx),
                            TabVisible::ImageReader => self.render_image_reader_tab(&self.visible_groupings[2..4], ctx),
                        }
                    }

                    // TODO: Possibly something to do with it being "default settings"

                    <button class="btn btn-success" onclick={ ctx.link().callback(|_| Msg::Submit) }>{ "Submit" }</button>
                </div>
            </div>
        }
    }
}

impl MemberGeneralPage {
    fn render_image_reader_tab(&self, visible: &[bool], ctx: &Context<Self>) -> Html {
        html! {
            <>
                // Desktop
                <div onclick={ ctx.link().callback(|_| Msg::ToggleGrouping(0)) }>
                    <span class="fs-2">{ "Desktop" }</span>
                    {
                        if visible[0] {
                            html! {
                                <span class="ms-2">{ "(close)" }</span>
                            }
                        } else {
                            html! {
                                <span class="ms-2">{ "(open)" }</span>
                            }
                        }
                    }
                </div>
                {
                    if visible[0] {
                        html! {
                            <>
                                <hr/>
                                { Self::render_image_group(EditingType::Desktop, &self.preferences.image_book.desktop.image, ctx) }
                                { Self::render_general_group(EditingType::Desktop, &self.preferences.image_book.desktop.general, ctx) }
                            </>
                        }
                    } else {
                        html! {}
                    }
                }

                // Mobile & Tablet
                <div onclick={ ctx.link().callback(|_| Msg::ToggleGrouping(1)) }>
                    <span class="fs-2">{ "Mobile & Tablet" }</span>
                    {
                        if visible[1] {
                            html! {
                                <span class="ms-2">{ "(close)" }</span>
                            }
                        } else {
                            html! {
                                <span class="ms-2">{ "(open)" }</span>
                            }
                        }
                    }
                </div>
                {
                    if visible[1] {
                        html! {
                            <>
                                <hr/>
                                { Self::render_image_group(EditingType::Mobile, &self.preferences.image_book.mobile.image, ctx) }
                                { Self::render_general_group(EditingType::Mobile, &self.preferences.image_book.mobile.general, ctx) }
                            </>
                        }
                    } else {
                        html! {}
                    }
                }
            </>
        }
    }

    fn render_text_reader_tab(&self, visible: &[bool], ctx: &Context<Self>) -> Html {
        html! {
            <>
                // Desktop
                <div onclick={ ctx.link().callback(|_| Msg::ToggleGrouping(0)) }>
                    <span class="fs-2">{ "Desktop" }</span>
                    {
                        if visible[0] {
                            html! {
                                <span class="ms-2">{ "(close)" }</span>
                            }
                        } else {
                            html! {
                                <span class="ms-2">{ "(open)" }</span>
                            }
                        }
                    }
                </div>
                {
                    if visible[0] {
                        html! {
                            <>
                                <hr/>
                                { Self::render_general_group(EditingType::Desktop, &self.preferences.text_book.desktop.general, ctx) }
                            </>
                        }
                    } else {
                        html! {}
                    }
                }

                // Mobile & Tablet
                <div onclick={ ctx.link().callback(|_| Msg::ToggleGrouping(1)) }>
                    <span class="fs-2">{ "Mobile & Tablet" }</span>
                    {
                        if visible[1] {
                            html! {
                                <span class="ms-2">{ "(close)" }</span>
                            }
                        } else {
                            html! {
                                <span class="ms-2">{ "(open)" }</span>
                            }
                        }
                    }
                </div>
                {
                    if visible[1] {
                        html! {
                            <>
                                <hr/>
                                { Self::render_general_group(EditingType::Mobile, &self.preferences.text_book.mobile.general, ctx) }
                            </>
                        }
                    } else {
                        html! {}
                    }
                }
            </>
        }
    }

    fn render_image_group(
        editing: EditingType,
        prefs: &ReaderImagePreferences,
        ctx: &Context<Self>,
    ) -> Html {
        html! {
            //
        }
    }

    fn render_general_group(
        editing: EditingType,
        reader: &GeneralBookPreferences,
        ctx: &Context<Self>,
    ) -> Html {
        html! {
            <>
                <h4>{ "General Settings" }</h4>

                // Auto Full screen?
                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox"
                        checked={ reader.auto_full_screen }
                        onchange={ ctx.link().callback(move |event: Event| {
                            Msg::UpdateSettingsGeneral(
                                editing,
                                Box::new(|pref, value| pref.auto_full_screen = value.as_bool().unwrap()),
                                serde_json::Value::Bool(event.target_unchecked_into::<HtmlInputElement>().checked())
                            )
                        }) }
                    />
                    <label class="form-check-label">{ "Fullscreen Reader" }</label>
                </div>

                // Animate Page Transitions?
                <div class="mb-3 form-check">
                    <input class="form-check-input" type="checkbox"
                        checked={ reader.animate_page_transitions }
                        onchange={ ctx.link().callback(move |event: Event| {
                            Msg::UpdateSettingsGeneral(
                                editing,
                                Box::new(|pref, value| pref.animate_page_transitions = value.as_bool().unwrap()),
                                serde_json::Value::Bool(event.target_unchecked_into::<HtmlInputElement>().checked())
                            )
                        }) }
                    />
                    <label class="form-check-label">{ "Animate page transitions" }</label>
                </div>

                // Reader Color
                <div class="mb-3">
                    <label class="form-label">{ "Reader Color" }</label>
                    <SelectModule<ReaderColor>
                        default={ reader.bg_color.clone() }
                        class="form-select"
                        onselect={ ctx.link().callback(move |value: ReaderColor| {
                            Msg::UpdateSettingsGeneral(
                                editing,
                                Box::new(move |pref, _| pref.bg_color = value.clone()),
                                serde_json::Value::Null
                            )
                        }) }
                    >
                        <SelectItem<ReaderColor> value={ ReaderColor::Default } name="Default" />
                        <SelectItem<ReaderColor> value={ ReaderColor::Black } name="Black" />
                    </SelectModule<ReaderColor>>
                </div>

                // Reader Display
                <div class="mb-3">
                    <label class="form-label">{ "Reader Display" }</label>
                    <SelectModule<LayoutType>
                        default={ reader.display_type }
                        class="form-select"
                        onselect={ ctx.link().callback(move |value| {
                            Msg::UpdateSettingsGeneral(
                                editing,
                                Box::new(move |pref, _| pref.display_type = value),
                                serde_json::Value::Null
                            )
                        }) }
                    >
                        <SelectItem<LayoutType> value={ LayoutType::Single } name="Single Page" />
                        <SelectItem<LayoutType> value={ LayoutType::Double } name="Double Page" />
                        <SelectItem<LayoutType> value={ LayoutType::Scroll } name="Scrolling Page" />
                    </SelectModule<LayoutType>>
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
                    // ReaderLoadType
                    <label class="form-label">{ "Load Sections By" }</label>
                    <SelectModule<ReaderLoadType>
                        disabled=true
                        default={ reader.load_type }
                        class="form-select"
                        onselect={ ctx.link().callback(move |value| {
                            Msg::UpdateSettingsGeneral(
                                editing,
                                Box::new(move |pref, _| pref.load_type = value),
                                serde_json::Value::Null
                            )
                        }) }
                    >
                        <SelectItem<ReaderLoadType> value={ ReaderLoadType::All } name="All" />
                        <SelectItem<ReaderLoadType> value={ ReaderLoadType::Select } name="Select" />
                    </SelectModule<ReaderLoadType>>
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
