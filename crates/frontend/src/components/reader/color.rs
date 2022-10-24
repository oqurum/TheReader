use wasm_bindgen::{UnwrapThrowExt, JsCast};
use web_sys::HtmlElement;

use super::section::SectionContents;


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



pub fn for_each_child<V: Fn(&HtmlElement)>(element: &HtmlElement, func: &V) {
    let children = element.children();

    for i in 0..children.length() {
        let child = children.item(i).unwrap_throw();
        let child: HtmlElement = child.unchecked_into();

        func(&child);

        for_each_child(&child, func);
    }
}