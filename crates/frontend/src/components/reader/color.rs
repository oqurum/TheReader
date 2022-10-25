use wasm_bindgen::UnwrapThrowExt;

use super::{section::SectionContents, util::for_each_child};

const STYLE: &str = "background: none !important; font-family: 'Roboto', sans-serif !important; color: #c9c9c9 !important;";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SectionColor {
    Default,
    Black,
    // White,
    // Custom {  },
}

impl SectionColor {
    pub fn on_load(self, section: &SectionContents) {
        match self {
            Self::Default => (),
            Self::Black => {
                let body = section.get_iframe_body().unwrap_throw();

                body.class_list().add_1("color-black").unwrap_throw();

                // FIX: For some reason the inline CSS will not be the top priority.
                for_each_child(&body, &|v| v.style().set_css_text(STYLE));
            }
        }
    }
}
