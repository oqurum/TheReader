use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlElement, Node};

pub mod selection;
pub mod table;

pub fn for_each_child_node<V: Fn(&Node) + Clone + Copy + 'static>(element: &Node, func: V) {
    let children = element.child_nodes();
    let mut index = 0;

    while let Some(child) = children.item(index) {
        func(&child);

        for_each_child_node(&child, func);

        index += 1;
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

pub fn for_each_child_map<E, V: Fn(&HtmlElement) -> Option<E>>(
    element: &HtmlElement,
    func: &V,
) -> Vec<E> {
    fn inner_map<E, V: Fn(&HtmlElement) -> Option<E>>(
        element: &HtmlElement,
        func: &V,
        items: &mut Vec<E>,
    ) {
        let children = element.children();

        for i in 0..children.length() {
            let child = children.item(i).unwrap_throw();
            let child: HtmlElement = child.unchecked_into();

            if let Some(v) = func(&child) {
                items.push(v);
            }

            items.extend(for_each_child_map(&child, func));
        }
    }

    let mut items = Vec::new();

    if let Some(v) = func(element) {
        items.push(v);
    }

    inner_map(element, func, &mut items);

    items
}

pub fn for_each_sibling_until<V: Fn(&HtmlElement) -> bool>(element: &HtmlElement, func: &V) {
    let mut sibling = element.next_element_sibling();

    while let Some(node) = sibling {
        let child: HtmlElement = node.unchecked_into();

        if !func(&child) {
            break;
        }

        sibling = child.next_element_sibling();
    }
}
