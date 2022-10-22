use common::component::{Popup, PopupType};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::{html::Scope, prelude::*};

use crate::components::reader::{PageLoadType, ReaderSettings};

use super::{Msg, ReadingBook};

pub const DEFAULT_DIMENSIONS: (i32, i32) = (1040, 548);

#[derive(Properties)]
pub struct SettingsContainerProps {
    pub scope: Scope<ReadingBook>,

    pub reader_dimensions: (i32, i32),

    pub reader_settings: ReaderSettings,
}

impl PartialEq for SettingsContainerProps {
    fn eq(&self, other: &Self) -> bool {
        self.reader_dimensions == other.reader_dimensions
            && self.reader_settings == other.reader_settings
    }
}

#[function_component(SettingsContainer)]
pub fn _settings_cont(props: &SettingsContainerProps) -> Html {
    let settings = props.reader_settings.clone();
    let settings = use_mut_ref(move || settings);

    let page_load_type_section = {
        let settings_inner = settings.clone();

        html! {
            <div class="form-container shrink-width-to-content">
                <label for="page-load-select">{ "Page Load Type" }</label>

                <select id="page-load-select"
                    onchange={ Callback::from(move |e: Event| {
                        let idx = e.target().unwrap()
                            .unchecked_into::<HtmlSelectElement>()
                            .selected_index();

                        match idx {
                            0 => settings_inner.borrow_mut().type_of = PageLoadType::All,
                            1 => settings_inner.borrow_mut().type_of = PageLoadType::Select,

                            _ => ()
                        }
                    })
                }>
                    <option selected={ settings.borrow().type_of == PageLoadType::All }>{ "Load All" }</option>
                    <option selected={ settings.borrow().type_of == PageLoadType::Select }>{ "Load When Needed" }</option>
                </select>
            </div>
        }
    };

    let screen_size_type_section = {
        let settings_inner = settings.clone();

        html! {
            <div class="form-container shrink-width-to-content">
                <label for="screen-size-select">{ "Screen Size Selection" }</label>

                <select id="screen-size-select"
                    onchange={ Callback::from(move |e: Event| {
                        let idx = e.target().unwrap()
                            .unchecked_into::<HtmlSelectElement>()
                            .selected_index();

                        let mut inner = settings_inner.borrow_mut();
                        inner.is_fullscreen = idx != 0;

                        if !inner.is_fullscreen {
                            inner.dimensions = DEFAULT_DIMENSIONS;
                        }

                    })
                }>
                    <option selected={ !settings.borrow().is_fullscreen }>{ "Specified" }</option>
                    <option selected={ settings.borrow().is_fullscreen }>{ "Full screen" }</option>
                </select>
            </div>
        }
    };

    let screen_size_section = {
        if settings.borrow().is_fullscreen {
            html! {}
        } else {
            let settings = settings.clone();

            let ref_width_input = use_node_ref();
            let ref_height_input = use_node_ref();

            html! {
                <div class="form-container shrink-width-to-content">
                    <label>{ "Screen Width and Height" }</label>

                    <div>
                        <input
                            style="width: 100px;"
                            value={ props.reader_dimensions.0.to_string() }
                            ref={ ref_width_input.clone() }
                            type="number"
                        />

                        <span>{ "x" }</span>

                        <input
                            style="width: 100px;"
                            value={ props.reader_dimensions.1.to_string() }
                            ref={ ref_height_input.clone() }
                            type="number"
                        />
                    </div>

                    <button onclick={ Callback::from(move |_| {
                        let width = ref_width_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;
                        let height = ref_height_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;

                        settings.borrow_mut().dimensions = (width, height);
                    }) }>{ "Update Dimensions" }</button>
                </div>
            }
        }
    };

    let reader_view_type_section = {
        let settings_inner = settings.clone();

        html! {
            <div class="form-container shrink-width-to-content">
                <label for="page-type-select">{ "Reader View Type" }</label>
                <select id="page-type-select" onchange={
                    Callback::from(move |e: Event| {
                        let display = e.target().unwrap()
                            .unchecked_into::<HtmlSelectElement>()
                            .value()
                            .parse::<u8>().unwrap()
                            .into();

                        settings_inner.borrow_mut().display = display;
                    })
                }>
                    <option value="0" selected={ settings.borrow().display.is_single() }>{ "Single Page" }</option>
                    <option value="1" selected={ settings.borrow().display.is_double() }>{ "Double Page" }</option>
                    <option value="2" selected={ settings.borrow().display.is_scroll() }>{ "Scrolling Page" }</option>
                </select>
            </div>
        }
    };

    html! {
        <Popup type_of={ PopupType::FullOverlay } on_close={ props.scope.callback(|_| Msg::ClosePopup) }>
            <div class="settings">
                { page_load_type_section }

                { screen_size_type_section }

                { screen_size_section }

                { reader_view_type_section }

                <hr />

                <div>
                    <button
                        class="green"
                        onclick={ props.scope.callback(move |_| Msg::ChangeReaderSettings(settings.take())) }
                    >{ "Submit" }</button>
                </div>
            </div>
        </Popup>
    }
}
