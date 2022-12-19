use common_local::reader::ReaderColor;
use wasm_bindgen::UnwrapThrowExt;

use super::{section::SectionContents, util::for_each_child};

const STYLE: &str = "background: none !important; font-family: 'Roboto', sans-serif !important; color: #c9c9c9 !important;";

pub fn load_reader_color_into_section(this: &ReaderColor, section: &SectionContents) {
    match this {
        // TODO: Determine if there is no background color. We'll need to add one.
        ReaderColor::Default => (),

        ReaderColor::Black => {
            let body = section.get_iframe_body().unwrap_throw();

            body.class_list().add_1("color-black").unwrap_throw();

            // FIX: For some reason the inline CSS will not be the top priority.
            for_each_child(&body, &|v| v.style().set_css_text(STYLE));
        }

        v => unreachable!("{v:?}"),
    }
}
