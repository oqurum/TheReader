use wasm_bindgen::{UnwrapThrowExt, JsCast};
use web_sys::HtmlElement;

pub mod table;

pub fn for_each_child<V: Fn(&HtmlElement)>(element: &HtmlElement, func: &V) {
    let children = element.children();

    for i in 0..children.length() {
        let child = children.item(i).unwrap_throw();
        let child: HtmlElement = child.unchecked_into();

        func(&child);

        for_each_child(&child, func);
    }
}
