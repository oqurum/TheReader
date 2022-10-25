use wasm_bindgen::{JsCast, UnwrapThrowExt};
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

pub fn for_each_child_map<E, V: Fn(&HtmlElement) -> Option<E>>(
    element: &HtmlElement,
    func: &V,
) -> Vec<E> {
    let children = element.children();

    let mut items = Vec::new();

    for i in 0..children.length() {
        let child = children.item(i).unwrap_throw();
        let child: HtmlElement = child.unchecked_into();

        if let Some(v) = func(&child) {
            items.push(v);
        }

        items.extend(for_each_child_map(&child, func));
    }

    items
}
