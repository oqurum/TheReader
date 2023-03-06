//! A Section is a collection of chapters that are displayed in a single iframe.
//!
//! For Example, this'll contain an iframe along with a collection of "chapters" which have the same header hash.

use std::rc::Rc;

use common_local::Chapter;
use wasm_bindgen::{prelude::Closure, UnwrapThrowExt};
use web_sys::{HtmlElement, HtmlIFrameElement, HtmlHeadElement};
use yew::Context;

use super::{
    js_update_iframe_after_load, update_iframe_size,
    util::{for_each_child_map, table::TableContainer},
    CachedPage, Reader, color, ReaderSettings,
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
            SectionLoadProgress::Waiting => panic!("Shouldn't have tried to convert a waiting variant"),
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

pub struct SectionContents {
    #[allow(dead_code)]
    on_load: Closure<dyn FnMut()>,

    chapters: Vec<Rc<Chapter>>,

    cached_pages: Vec<CachedPage>,

    iframe: HtmlIFrameElement,

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
    pub fn new(header_hash: String, iframe: HtmlIFrameElement, on_load: Closure<dyn FnMut()>) -> Self {
        Self {
            iframe,
            on_load,
            header_hash,
            chapters: Vec::new(),
            cached_pages: Vec::new(),
            gpi: 0,
            page_offset: 0,
            cached_tables: Vec::new(),
        }
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

    pub fn on_load(
        &mut self,
        handle_js_redirect_clicks: &Closure<dyn FnMut(String, String)>,
        settings: &mut ReaderSettings,
        ctx: &Context<Reader>,
    ) {
        // Insert chapters
        {
            let doc = self.get_iframe().content_document().unwrap_throw();
            let body = self.get_iframe_body().unwrap_throw();
            let mut inserted_header = false;

            for chapter in &self.chapters {
                //  Insert Initial Header
                if !inserted_header {
                    let head = self.get_iframe_head().unwrap_throw();

                    for item in &chapter.info.header_items {
                        if item.name.to_lowercase() == "link" {
                            let link = doc.create_element("link").unwrap_throw();

                            for attr in &item.attributes {
                                link.set_attribute(&attr.0, &attr.1).unwrap_throw();
                            }

                            head.append_with_node_1(&link).unwrap_throw();
                        } else if item.name.to_lowercase() == "style" {
                            let style = doc.create_element("style").unwrap_throw();

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
                    let section_break = doc.create_element("div").unwrap_throw();
                    section_break.set_attribute("data-section-id", &chapter.value.to_string()).unwrap_throw();
                    section_break.class_list().add_2("reader-ignore", "reader-section-start").unwrap_throw();
                    section_break.set_id(&format!("section-{}-start", chapter.value));
                    body.append_child(&section_break).unwrap_throw();

                    section_break.insert_adjacent_html("afterend", chapter.info.inner_body.trim()).unwrap_throw();

                    // End of Section Declaration
                    let section_break = doc.create_element("div").unwrap_throw();
                    section_break.set_attribute("data-section-id", &chapter.value.to_string()).unwrap_throw();
                    section_break.class_list().add_2("reader-ignore", "reader-section-end").unwrap_throw();
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

        js_update_iframe_after_load(self.get_iframe(), &self.header_hash, handle_js_redirect_clicks);

        settings.display.add_to_iframe(self.get_iframe(), ctx);
        settings.display.on_stop_viewing(self);

        update_iframe_size(Some(settings.dimensions), self.get_iframe());

        // TODO: Detect if there's a single image in the whole section. If so, expand across both page views and center.
    }
}

impl PartialEq for SectionContents {
    fn eq(&self, other: &Self) -> bool {
        self.header_hash == other.header_hash
    }
}
