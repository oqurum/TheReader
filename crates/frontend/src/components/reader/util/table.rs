//! The Table Splitter used for the reader.

use js_sys::Array;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlElement, Node};

// TODO: Implement ability to rejoin Table Splits.

pub enum TableContainer {
    Single(Table),
    Split(Vec<Table>),
}

impl TableContainer {
    pub fn update_if_needed(&mut self, max_height: usize) {
        self.split_and_fit(max_height);
    }

    fn split_and_fit(&mut self, max_height: usize) {
        match self {
            Self::Single(v) => {
                let new_tables = v.split_and_fit(max_height);

                // Replace Variant with Split.
                if !new_tables.is_empty() {
                    let table =
                        if let Self::Single(v) = std::mem::replace(self, Self::Split(new_tables)) {
                            v
                        } else {
                            todo!()
                        };

                    if let Self::Split(values) = self {
                        // Prepend the original table.
                        values.insert(0, table);
                    }
                }
            }

            Self::Split(tables) => {
                for table in std::mem::take(tables) {
                    let mut new_tables = table.split_and_fit(max_height);

                    tables.push(table);
                    tables.append(&mut new_tables);
                }
            }
        }
    }
}

impl From<HtmlElement> for TableContainer {
    fn from(value: HtmlElement) -> Self {
        Self::Single(Table::from(value))
    }
}

impl From<&HtmlElement> for TableContainer {
    fn from(value: &HtmlElement) -> Self {
        Self::Single(Table::from(value))
    }
}

#[derive(Clone)]
pub struct Table {
    pub container: HtmlElement,
    pub items: Vec<HtmlElement>,
}

impl Table {
    fn get_contents_of_table(container: HtmlElement) -> Self {
        let found = Array::from(&container.children());

        let items = found
            .iter()
            .map(|v| v.unchecked_into())
            .collect::<Vec<HtmlElement>>();

        Self { container, items }
    }

    /// Splits and fits the table into the reader
    fn split_and_fit(&self, max_height: usize) -> Vec<Self> {
        // Causes Reflow
        let table_rect = self.container.get_bounding_client_rect();

        if table_rect.height().ceil() as usize > max_height {
            if self.contains_body() {
                if self.contains_multiple_bodies() {
                    log::error!("Currently not handling multiple bodies.");
                } else if let Some(body) = self.items.iter().find(|v| v.local_name() == "tbody") {
                    let body_rect = body.get_bounding_client_rect();

                    // Get size before and after body. Used so we know what the initial size of the next table is without calling "get_bounding_client_rect".
                    let size_before = body_rect.top() - table_rect.top();
                    let size_after = body_rect.bottom() - table_rect.bottom();

                    let mut height_left_to_remove = table_rect.height() - max_height as f64;

                    let mut moving_children = Vec::new();

                    // Children should be <tr> -- Removes the children from Back to Front until we shouldn't remove any more.
                    Array::from(&body.children()).iter().rev().for_each(|v| {
                        let item: HtmlElement = v.unchecked_into();

                        let rect = item.get_bounding_client_rect();

                        // If we want to remove more OR we've only moved 1 child.
                        if height_left_to_remove > 0.0 || moving_children.len() == 1 {
                            item.remove();
                            height_left_to_remove -= rect.height();

                            moving_children.push((rect.height(), item));
                        }
                    });

                    // Reverse to use .pop()
                    moving_children.reverse();

                    let mut new_tables = Vec::new();

                    while !moving_children.is_empty() {
                        let new_table = Self::get_contents_of_table(
                            self.container
                                .clone_node_with_deep(true)
                                .unwrap_throw()
                                .unchecked_into(),
                        );

                        let mut table_size = size_before + size_after;

                        new_table.clear();

                        while let Some((size, node)) = moving_children.pop() {
                            table_size += size;

                            if table_size > max_height as f64 {
                                moving_children.push((size, node));
                                break;
                            } else {
                                new_table.prepend(&node);
                            }
                        }

                        if new_tables.is_empty() {
                            new_table.insert_after(self);
                        } else {
                            new_table.insert_after(&new_tables[new_tables.len() - 1]);
                        }

                        new_tables.push(new_table);
                    }

                    return new_tables;
                }
            } else {
                log::error!("Doesn't contain body");
            }
        }

        Vec::new()
    }

    pub fn insert_after(&self, other: &Self) {
        other
            .container
            .after_with_node_1(&self.container)
            .unwrap_throw();
    }

    fn contains_body(&self) -> bool {
        self.items.iter().any(|v| v.local_name() == "tbody")
    }

    fn contains_multiple_bodies(&self) -> bool {
        self.items
            .iter()
            .filter(|v| v.local_name() == "tbody")
            .count()
            > 1
    }

    fn clear(&self) {
        if self.contains_body() {
            if self.contains_multiple_bodies() {
                log::error!("Currently not handling multiple bodies.");
            } else if let Some(body) = self.items.iter().find(|v| v.local_name() == "tbody") {
                while let Some(child) = body.first_child() {
                    body.remove_child(&child).unwrap_throw();
                }
            }
        } else {
            log::error!("Doesn't contain body");
        }
    }

    fn prepend(&self, node: &Node) {
        if self.contains_body() {
            if self.contains_multiple_bodies() {
                log::error!("Currently not handling multiple bodies.");
            } else if let Some(body) = self.items.iter().find(|v| v.local_name() == "tbody") {
                body.prepend_with_node_1(node).unwrap_throw();
            }
        } else {
            log::error!("Doesn't contain body");
        }
    }
}

impl From<HtmlElement> for Table {
    fn from(value: HtmlElement) -> Self {
        Self::get_contents_of_table(value)
    }
}

impl From<&HtmlElement> for Table {
    fn from(value: &HtmlElement) -> Self {
        Self::get_contents_of_table(value.clone())
    }
}
