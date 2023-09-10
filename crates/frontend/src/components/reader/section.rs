//! A Section is a collection of chapters that are displayed in a single iframe.
//!
//! For Example, this'll contain an iframe along with a collection of "chapters" which have the same header hash.

use std::{cell::RefCell, rc::Rc};

use common_local::Chapter;
use editor::{ListenerEvent, ListenerHandle, ListenerId, MouseListener};
use gloo_utils::window;
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::{Document, Element, HtmlElement, HtmlHeadElement, HtmlIFrameElement, Node};
use yew::Context;

use super::{
    color, js_update_iframe_after_load, update_iframe_size,
    util::{for_each_child_map, table::TableContainer},
    CachedPage, LayoutDisplay, Reader, ReaderSettings,
};

pub enum SectionLoadProgress {
    Waiting,
    Loading(SectionContents),
    Loaded(SectionContents),
}

impl SectionLoadProgress {
    pub fn is_waiting(&self) -> bool {
        matches!(self, Self::Waiting)
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    pub fn convert_to_loaded(&mut self) {
        match std::mem::replace(self, Self::Waiting) {
            SectionLoadProgress::Loading(v) => *self = Self::Loaded(v),
            SectionLoadProgress::Loaded(v) => *self = Self::Loaded(v),
            SectionLoadProgress::Waiting => {
                panic!("Shouldn't have tried to convert a waiting variant")
            }
        }
    }

    pub fn as_loaded(&self) -> Option<&SectionContents> {
        match self {
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_chapter(&self) -> Option<&SectionContents> {
        match self {
            Self::Loading(v) | Self::Loaded(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_chapter_mut(&mut self) -> Option<&mut SectionContents> {
        match self {
            Self::Loading(v) | Self::Loaded(v) => Some(v),
            _ => None,
        }
    }
}

/// Used to manage an iframe and multiple chapters which are the same header hash.
pub struct SectionContents {
    #[allow(dead_code)]
    on_load: Closure<dyn FnMut()>,

    chapters: Vec<Rc<Chapter>>,

    cached_pages: Vec<CachedPage>,

    iframe: HtmlIFrameElement,

    pub editor_handle: ListenerHandle,

    /// Global Page Index
    pub gpi: usize,

    /// Page offset for the current chapter
    ///
    /// We call this the "page offset" because it can be displaying a different page for RTL or LTR reading.
    pub page_offset: usize,

    cached_tables: Vec<TableContainer>,

    pub header_hash: String,
}

impl SectionContents {
    pub fn new(
        header_hash: String,
        iframe: HtmlIFrameElement,
        on_load: Closure<dyn FnMut()>,
    ) -> Self {
        Self {
            iframe,
            on_load,
            header_hash,
            chapters: Vec::new(),
            cached_pages: Vec::new(),
            editor_handle: ListenerHandle::unset(),
            gpi: 0,
            page_offset: 0,
            cached_tables: Vec::new(),
        }
    }

    /// Based upon relative coords
    pub fn get_element_at(&self, x: f32, y: f32) -> Option<Element> {
        let frame = self.get_iframe();

        let document = frame.content_document().unwrap();

        document.element_from_point(x, y)
    }

    pub fn find_elements(&self, selector: &str) -> Vec<Element> {
        let frame = self.get_iframe();

        let document = frame.content_document().unwrap();

        let nodes = document.query_selector_all(selector).unwrap();

        let mut items = Vec::new();

        for i in 0..nodes.length() as usize {
            items.push(nodes.item(i as u32).unwrap().unchecked_into());
        }

        items
    }

    // TODO: Handle images. Potentially add +100 bytes for each image?
    pub fn get_page_from_byte_position(&self, position: usize) -> Option<usize> {
        fn find_text_pos(
            byte_count: &mut usize,
            position: usize,
            cont: Node,
            document: &Document,
        ) -> Option<usize> {
            let value = cont.node_value().unwrap_or_default();

            if cont.node_type() == Node::TEXT_NODE && !value.trim().is_empty() {
                *byte_count += value.len();

                if *byte_count > position {
                    let body = document.body().unwrap().get_bounding_client_rect();
                    // We need to parent element since we're inside a Text Node.
                    let node = cont
                        .parent_node()
                        .unwrap()
                        .unchecked_into::<Element>()
                        .get_bounding_client_rect();

                    return Some(
                        ((body.x().abs()
                            + node.x()
                            + node.width() / 2.0
                            + (node.x() / body.width()).ceil() * 10.0)
                            / body.width())
                        .abs()
                        .floor() as usize,
                    );
                }
            }

            let nodes = cont.child_nodes();
            for index in 0..nodes.length() as usize {
                let node = nodes.item(index as u32).unwrap();
                let found = find_text_pos(byte_count, position, node, document);

                if found.is_some() {
                    return found;
                }
            }

            None
        }

        find_text_pos(
            &mut 0,
            position,
            self.get_iframe_body().unwrap().unchecked_into(),
            &self.get_iframe().content_document().unwrap(),
        )
    }

    // TODO: Handle images. Potentially add +100 bytes for each image?
    pub fn get_current_byte_pos(&self, is_vertical: bool) -> Option<(usize, usize)> {
        fn find_text_pos(
            moved_amount: f32,
            byte_count: &mut usize,
            last_section_id: &mut usize,
            is_vertical: bool,
            cont: Node,
        ) -> Option<(usize, usize)> {
            if cont.node_type() == Node::ELEMENT_NODE
                && (cont
                    .unchecked_ref::<HtmlElement>()
                    .class_list()
                    .contains("reader-section-start")
                    || cont
                        .unchecked_ref::<HtmlElement>()
                        .class_list()
                        .contains("reader-section-end"))
            {
                *last_section_id = cont
                    .unchecked_ref::<Element>()
                    .get_attribute("data-section-id")
                    .unwrap()
                    .parse()
                    .unwrap();
            }

            if cont.node_type() == Node::TEXT_NODE
                && !cont.node_value().unwrap_or_default().trim().is_empty()
            {
                // TODO: Will probably mess up if element takes up a full page.
                if (is_vertical
                    && (moved_amount
                        - (cont
                            .parent_element()
                            .unwrap()
                            .unchecked_into::<HtmlElement>()
                            .offset_top() as f32)
                        < 0.0))
                    || (!is_vertical
                        && (moved_amount
                            - (cont
                                .parent_element()
                                .unwrap()
                                .unchecked_into::<HtmlElement>()
                                .offset_left() as f32)
                            < 0.0))
                {
                    return Some((*byte_count, *last_section_id));
                } else {
                    *byte_count += cont.node_value().unwrap().len();
                    // TODO: Check if we overshot the view page. If so half it and check again
                }
            }

            let nodes = cont.child_nodes();
            for index in 0..nodes.length() as usize {
                let node = nodes.item(index as u32).unwrap();
                let found =
                    find_text_pos(moved_amount, byte_count, last_section_id, is_vertical, node);

                if found.is_some() {
                    return found;
                }
            }

            None
        }

        // How much we've moved our current view.
        let amount;

        if is_vertical {
            amount = self.get_iframe_body().unwrap().scroll_top() as f32;
        } else {
            let cs = window()
                .get_computed_style(&self.get_iframe_body().unwrap())
                .unwrap()
                .unwrap();

            amount = cs
                .get_property_value("left")
                .unwrap()
                .replace("px", "")
                .parse::<f32>()
                .unwrap()
                .abs();
        }

        find_text_pos(
            amount,
            &mut 0,
            &mut 0,
            is_vertical,
            self.get_iframe_body().unwrap().unchecked_into(),
        )
    }

    pub fn get_iframe(&self) -> &HtmlIFrameElement {
        &self.iframe
    }

    pub fn get_iframe_body(&self) -> Option<HtmlElement> {
        self.get_iframe().content_document()?.body()
    }

    pub fn get_iframe_head(&self) -> Option<HtmlHeadElement> {
        self.get_iframe().content_document()?.head()
    }

    pub fn append_chapter(&mut self, value: Rc<Chapter>) {
        self.chapters.push(value);
    }

    pub fn get_chapters(&self) -> &[Rc<Chapter>] {
        &self.chapters
    }

    pub fn set_cached_pages(&mut self, value: Vec<CachedPage>) {
        self.cached_pages = value;
    }

    pub fn page_count(&self) -> usize {
        self.cached_pages.len()
    }

    pub fn get_page_count_until(&self) -> usize {
        self.gpi + self.page_count()
    }

    /// Called after the iframe has fully loaded.
    pub fn after_load(
        &mut self,
        handle_js_redirect_clicks: &Closure<dyn FnMut(String, String)>,
        settings: &ReaderSettings,
        cached_display: &mut LayoutDisplay,
        ctx: &Context<Reader>,
    ) {
        let document = self.get_iframe().content_document().unwrap_throw();

        // Insert chapters
        {
            let body = self.get_iframe_body().unwrap_throw();
            let mut inserted_header = false;

            for chapter in &self.chapters {
                //  Insert Initial Header
                if !inserted_header {
                    let head = self.get_iframe_head().unwrap_throw();

                    for item in &chapter.info.header_items {
                        if item.name.to_lowercase() == "link" {
                            let link = document.create_element("link").unwrap_throw();

                            for attr in &item.attributes {
                                link.set_attribute(&attr.0, &attr.1).unwrap_throw();
                            }

                            head.append_with_node_1(&link).unwrap_throw();
                        } else if item.name.to_lowercase() == "style" {
                            let style = document.create_element("style").unwrap_throw();

                            for attr in &item.attributes {
                                style.set_attribute(&attr.0, &attr.1).unwrap_throw();
                            }

                            style.set_text_content(item.chars.as_deref());

                            head.append_with_node_1(&style).unwrap_throw();
                        }
                    }

                    inserted_header = true;
                }

                // Append to body
                {
                    // Start of Section Declaration
                    let section_break = document.create_element("div").unwrap_throw();
                    section_break
                        .set_attribute("data-section-id", &chapter.value.to_string())
                        .unwrap_throw();
                    section_break
                        .class_list()
                        .add_2("reader-ignore", "reader-section-start")
                        .unwrap_throw();
                    section_break.set_id(&format!("section-{}-start", chapter.value));
                    body.append_child(&section_break).unwrap_throw();

                    section_break
                        .insert_adjacent_html("afterend", chapter.info.inner_body.trim())
                        .unwrap_throw();

                    // End of Section Declaration
                    let section_break = document.create_element("div").unwrap_throw();
                    section_break
                        .set_attribute("data-section-id", &chapter.value.to_string())
                        .unwrap_throw();
                    section_break
                        .class_list()
                        .add_2("reader-ignore", "reader-section-end")
                        .unwrap_throw();
                    section_break.set_id(&format!("section-{}-end", chapter.value));
                    body.append_child(&section_break).unwrap_throw();
                }
            }
        }

        color::load_reader_color_into_section(&settings.color, self);

        // Shrink Tables
        self.cached_tables = for_each_child_map(&self.get_iframe_body().unwrap_throw(), &|v| {
            if v.local_name() == "table" {
                let mut v = TableContainer::from(v);
                v.update_if_needed(settings.dimensions.1 as usize);

                Some(v)
            } else {
                None
            }
        });

        js_update_iframe_after_load(
            self.get_iframe(),
            &self.header_hash,
            handle_js_redirect_clicks,
        );

        cached_display.add_to_iframe(self.get_iframe(), ctx);
        cached_display.on_stop_viewing(self);

        update_iframe_size(Some(settings.dimensions), self.get_iframe());

        self.editor_handle = editor::register(
            self.get_iframe_body().unwrap_throw(),
            MouseListener::Ignore,
            Some(document),
            Some(Rc::new(RefCell::new(move |_id: ListenerId| {
                // let save = id.try_save();
            })) as ListenerEvent),
        )
        .expect_throw("Failed to register editor listener");

        // TODO: Detect if there's a single image in the whole section. If so, expand across both page views and center.
    }
}

impl PartialEq for SectionContents {
    fn eq(&self, other: &Self) -> bool {
        self.header_hash == other.header_hash
    }
}
