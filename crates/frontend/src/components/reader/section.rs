use std::rc::Rc;

use common_local::Chapter;
use wasm_bindgen::{prelude::Closure, UnwrapThrowExt};
use web_sys::{HtmlElement, HtmlIFrameElement, HtmlHeadElement};
use yew::Context;

use super::{
    js_update_iframe_after_load, update_iframe_size,
    util::{for_each_child_map, table::TableContainer},
    CachedPage, Reader, SectionDisplay, color,
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
        if let Self::Loading(v) = std::mem::replace(self, Self::Waiting) {
            *self = Self::Loaded(v);
        } else {
            panic!("unable to convert")
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

    pub viewing_page: usize,

    cached_tables: Vec<TableContainer>,

    pub header_hash: String,
}

impl SectionContents {
    pub fn new(header_hash: String, iframe: HtmlIFrameElement, on_load: Closure<dyn FnMut()>) -> Self {
        Self {
            on_load,
            chapters: Vec::new(),
            cached_pages: Vec::new(),
            iframe,
            gpi: 0,
            viewing_page: 0,
            cached_tables: Vec::new(),
            header_hash,
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

    pub fn set_cached_pages(&mut self, value: Vec<CachedPage>) {
        self.cached_pages = value;
    }

    pub fn page_count(&self) -> usize {
        self.cached_pages.len()
    }

    pub fn viewing_page(&self) -> usize {
        self.viewing_page
    }

    pub fn get_page_count_until(&self) -> usize {
        self.gpi + self.page_count()
    }

    pub fn transitioning_page(&self, amount: isize) {
        let body = self.iframe.content_document().unwrap().body().unwrap();

        // Prevent empty pages when on the first or last page of a section.
        let amount = if (amount.is_positive() && self.viewing_page == 0)
            || (amount.is_negative() && self.viewing_page == self.page_count().saturating_sub(1))
        {
            0
        } else {
            amount
        };

        if amount == 0 {
            body.style()
                .set_property("transition", "left 0.5s ease 0s")
                .unwrap();
        } else {
            body.style().remove_property("transition").unwrap();
        }

        body.style()
            .set_property(
                "left",
                &format!(
                    "calc(-{}% - {}px)",
                    100 * self.viewing_page,
                    self.viewing_page as isize * 10 - amount
                ),
            )
            .unwrap();
    }

    // pub fn initial_load(&self, sections: &mut [SectionLoadProgress]) {
    //     // Insert Headers.

    //     // Insert body
    //     for section in sections {
    //         if let Some(cont) = section.as_chapter() {
    //             if cont.header_hash == self.header_hash {
    //                 self.insert_section(cont);
    //             }

    //             section.convert_to_loaded();
    //         }
    //     }
    // }

    // pub fn insert_section(&self, section: &SectionContents) {
    //     let doc = self.element.content_document().unwrap_throw();
    //     let body = self.get_body().unwrap_throw();

    //     {
    //         let section_break = doc.create_element("div").unwrap_throw();
    //         section_break.set_id(&format!("section-{}", section.chapter()));
    //         body.append_child(&section_break).unwrap_throw();
    //     }

    //     body.append_with_str_1("nodes_1");
    // }

    pub fn on_load(
        &mut self,
        cached_display: &mut SectionDisplay,
        handle_js_redirect_clicks: &Closure<dyn FnMut(String, String)>,
        ctx: &Context<Reader>,
    ) {
        // Insert chapters
        {
            let mut inserted_header = false;
            for chapter in &self.chapters {
                log::debug!("inserting chapter");
                if !inserted_header {
                    let doc = self.get_iframe().content_document().unwrap_throw();
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
            }
        }

        let settings = &ctx.props().settings;

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

        cached_display.add_to_iframe(self.get_iframe(), ctx);
        cached_display.on_stop_viewing(self);

        update_iframe_size(Some(settings.dimensions), self.get_iframe());

        // TODO: Detect if there's a single image in the whole section. If so, expand across both page views and center.
    }
}

impl PartialEq for SectionContents {
    fn eq(&self, other: &Self) -> bool {
        self.header_hash == other.header_hash
    }
}
