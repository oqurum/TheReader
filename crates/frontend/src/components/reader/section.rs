//! A Section is a collection of chapters that are displayed in a single iframe.
//!
//! For Example, this'll contain an iframe along with a collection of "chapters" which have the same header hash.

use std::{cell::RefCell, rc::Rc};

use common_local::Chapter;
use editor::{ListenerEvent, ListenerHandle, ListenerId, MouseListener};
use gloo_utils::{document, window};
use itertools::{FoldWhile, Itertools};
use js_sys::Array;
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::{DomRect, Element, HtmlElement, HtmlHeadElement, HtmlIFrameElement, Node, Text};
use yew::Context;

use super::{
    color, js_update_iframe_after_load,
    util::{for_each_child_map, for_each_child_node, table::TableContainer},
    CachedPage, LayoutDisplay, Reader, ReaderSettings,
};

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &js_sys::Object);
}

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

    pub chapters: Vec<Rc<Chapter>>,

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

    pub fn find_section_start(&self, index: usize) -> Option<Element> {
        let mut elements = self.find_elements(&format!("div[data-section-id=\"{index}\"]"));

        if elements.len() != 0 {
            Some(elements.swap_remove(0))
        } else {
            None
        }
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
            iframe: &HtmlIFrameElement,
        ) -> Option<usize> {
            if cont.node_type() == Node::TEXT_NODE {
                let value = cont.unchecked_ref::<Text>().data();

                if !value.trim().is_empty() {
                    *byte_count += value.len();

                    // TODO: Vertical pages
                    if *byte_count > position {
                        // We need to parent element since we're inside a Text Node.
                        let parent_element = cont
                            .parent_element()
                            .unwrap()
                            .unchecked_into::<HtmlElement>();

                        // If it spans multiple pages, offset should be negative b/c these are relative to current view.
                        // Will be positive if it's on a page to the right
                        let offset_x = Array::from(&parent_element.child_nodes())
                            .iter()
                            .map(|v| v.unchecked_into::<Node>())
                            .fold_while(f64::MAX, |v, n| {
                                if n == cont {
                                    // Incase we only had 1 node.
                                    if v == f64::MAX {
                                        FoldWhile::Done(0.0)
                                    } else {
                                        FoldWhile::Done(v)
                                    }
                                } else {
                                    let range = document().create_range().unwrap();
                                    range.select_node_contents(&n).unwrap();
                                    let rect = range.get_bounding_client_rect();

                                    FoldWhile::Continue(v.min(rect.x()))
                                }
                            })
                            .into_inner();

                        let iframe_width = iframe.client_width() as f32;

                        // Remove padding (node is text on right side of page) we only care about left side.
                        // 400 -> 0, 1,500 -> 1,000
                        let start_offset = (parent_element.offset_left() as f32 / iframe_width)
                            .floor()
                            * iframe_width;

                        let range = document().create_range().unwrap();
                        range.select_node_contents(&cont).unwrap();

                        // BB has relative positions
                        let x_s = range.get_bounding_client_rect().x() as f32;

                        let diff = x_s - offset_x as f32;

                        let calc_page = (diff + start_offset) / iframe_width;

                        // debug!(
                        //     "------- Page: {calc_page} || ox {offset_x}, s {start_offset}, r {x_s}, d {diff} | ",
                        // );
                        // debug!(
                        //     "------- ({diff} + {start_offset}) / {iframe_width} = {calc_page} || dif = ({x_s} - {offset_x})",
                        // );

                        // log(&cont);
                        // log(&parent_element);

                        return Some(calc_page.floor() as usize);
                    }
                }
            }

            let nodes = cont.child_nodes();
            for index in 0..nodes.length() as usize {
                let node = nodes.item(index as u32).unwrap();
                let found = find_text_pos(byte_count, position, node, iframe);

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
            self.get_iframe(),
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
            // Set current section id
            if cont.node_type() == Node::ELEMENT_NODE {
                let cont_element = cont.unchecked_ref::<HtmlElement>();
                let class_list = cont_element.class_list();

                if class_list.contains("reader-section-start")
                    || class_list.contains("reader-section-end")
                {
                    *last_section_id = cont_element
                        .get_attribute("data-section-id")
                        .unwrap()
                        .parse()
                        .unwrap();
                }
            }

            if cont.node_type() == Node::TEXT_NODE
                && !cont.node_value().unwrap_or_default().trim().is_empty()
            {
                let parent_element = cont
                    .parent_element()
                    .unwrap()
                    .unchecked_into::<HtmlElement>();

                // RESOLVES BUG: Messing up when an element takes up a full page.
                let start_offset = (is_vertical
                    .then(|| parent_element.offset_top())
                    .unwrap_or_else(|| parent_element.offset_left())
                    as f32)
                    - moved_amount;
                let end_offset = (is_vertical
                    .then(|| parent_element.offset_top() + parent_element.offset_height())
                    .unwrap_or_else(|| parent_element.offset_left() + parent_element.offset_width())
                    as f32)
                    - moved_amount;

                // If either is positive, the parent is in view
                if start_offset.is_sign_positive() || end_offset.is_sign_positive() {
                    let range = document().create_range().unwrap();
                    range.select_node_contents(&cont).unwrap();

                    if let Some(list) = range.get_client_rects() {
                        let (x_s, y_s) = Array::from(&list)
                            .iter()
                            .map(|v| v.unchecked_into::<DomRect>())
                            .fold((f64::MAX, f64::MAX), |(x, y), r| {
                                (x.min(r.x()), y.min(r.y()))
                            });

                        // TODO: Figure out a way to pick text halfway through on the left size of the page.
                        if (is_vertical && y_s.is_sign_positive())
                            || (!is_vertical && x_s.is_sign_positive())
                        {
                            debug!("b{byte_count} s{last_section_id}");
                            return Some((*byte_count, *last_section_id));
                        }
                    } else {
                        warn!("Unable to get client rects for some reason");
                    }
                }

                *byte_count += cont.node_value().unwrap().len();
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

        let body = self.get_iframe_body().unwrap_throw();

        // Insert chapters
        {
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

        // TODO: This is a temporary fix. We need to properly be able to determine byte position based off text which takes up multiple pages.
        // TODO: May have issues once we start highlighting things.
        // Split Text Nodes to be more digestible (byte position fix)
        for_each_child_node(&body, |node| {
            if node.node_type() == Node::TEXT_NODE {
                let mut text_node = node.clone().unchecked_into::<Text>();

                // TODO: Need a proper max length
                const MAX_LENGTH: u32 = 400;

                'woop: while text_node.length() > MAX_LENGTH {
                    let data = text_node.data();

                    let mut last_period_loc = 0;

                    // We split at the period to keep it simple.
                    for periods_loc in data.match_indices(".").map(|a| a.0) {
                        if periods_loc > MAX_LENGTH as usize {
                            if last_period_loc == 0 {
                                text_node = text_node.split_text(text_node.length() / 2).unwrap();
                            } else {
                                text_node =
                                    text_node.split_text(1 + last_period_loc as u32).unwrap();
                            }

                            continue 'woop;
                        }

                        last_period_loc = periods_loc;
                    }

                    // If there were no periods.
                    // MIN check to ensure the split text isn't greater than MAX_LENGTH
                    text_node = text_node
                        .split_text((text_node.length() / 2).min(MAX_LENGTH))
                        .unwrap();
                }
            }
        });

        // Shrink Tables
        self.cached_tables = for_each_child_map(&body, &|v| {
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

        self.editor_handle = editor::register(
            body,
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
