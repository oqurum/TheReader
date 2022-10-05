use wasm_bindgen::{prelude::Closure, UnwrapThrowExt, JsCast};
use web_sys::{HtmlIFrameElement, HtmlElement};

use super::{CachedPage, SectionDisplay};




pub enum SectionLoadProgress {
    Waiting,
    Loading(SectionContents),
    Loaded(SectionContents)
}

impl SectionLoadProgress {
    pub fn is_waiting(&self) -> bool {
        matches!(self, Self::Waiting)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading(_))
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    pub fn convert_to_loaded(self) -> Self {
        if let Self::Loading(v) = self {
            Self::Loaded(v)
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
            Self::Loading(v) |
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_chapter_mut(&mut self) -> Option<&mut SectionContents> {
        match self {
            Self::Loading(v) |
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }
}



pub struct SectionContents {
    #[allow(dead_code)]
    on_load: Closure<dyn FnMut()>,

    cached_pages: Vec<CachedPage>,

    iframe: HtmlIFrameElement,
    chapter: usize,

    /// Global Page Index
    pub gpi: usize,

    viewing_page: usize,
}

impl SectionContents {
    pub fn new(chapter: usize, iframe: HtmlIFrameElement, on_load: Closure<dyn FnMut()>) -> Self {
        Self {
            on_load,
            cached_pages: Vec::new(),
            iframe,
            chapter,
            gpi: 0,
            viewing_page: 0,
        }
    }


    pub fn get_iframe(&self) -> &HtmlIFrameElement {
        &self.iframe
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

    pub fn chapter(&self) -> usize {
        self.chapter
    }

    pub fn get_page_count_until(&self) -> usize {
        self.gpi + self.page_count()
    }


    pub fn transitioning_page(&self, amount: isize) {
        let body = self.iframe.content_document().unwrap().body().unwrap();

        // Prevent empty pages when on the first or last page of a section.
        let amount = if (amount.is_positive() && self.viewing_page == 0) ||
            (amount.is_negative() && self.viewing_page == self.page_count().saturating_sub(1))
        {
            0
        } else {
            amount
        };

        if amount == 0 {
            body.style().set_property("transition", "left 0.5s ease 0s").unwrap();
        } else {
            body.style().remove_property("transition").unwrap();
        }

        body.style().set_property(
            "left",
            &format!(
                "calc(-{}% - {}px)",
                100 * self.viewing_page,
                self.viewing_page as isize * 10 - amount
            )
        ).unwrap();
    }

    pub fn set_last_page(&mut self, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.scroll_with_x_and_y(0.0, el.scroll_height() as f64);
        } else {
            self.set_page(self.page_count().saturating_sub(1), display);
        }
    }

    pub fn set_page(&mut self, page_number: usize, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.scroll_with_x_and_y(0.0, 0.0);
        } else {
            self.viewing_page = page_number;

            let body = self.iframe.content_document().unwrap().body().unwrap();
            body.style().set_property("transition", "left 0.5s ease 0s").unwrap();
            body.style().set_property("left", &format!("calc(-{}% - {}px)", 100 * self.viewing_page, self.viewing_page * 10)).unwrap();
        }
    }

    pub fn next_page(&mut self, display: SectionDisplay) -> bool {
        if self.viewing_page + 1 < self.page_count() {
            self.set_page(self.viewing_page + 1, display);

            true
        } else {
            false
        }
    }

    pub fn previous_page(&mut self, display: SectionDisplay) -> bool {
        if self.viewing_page != 0 {
            self.set_page(self.viewing_page - 1, display);

            true
        } else {
            false
        }
    }


    pub fn on_stop_viewing(&self, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.style().set_property("overflow", "hidden").unwrap_throw();
        }
    }

    pub fn on_start_viewing(&self, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.style().remove_property("overflow").unwrap_throw();
        }
    }
}

impl PartialEq for SectionContents {
    fn eq(&self, other: &Self) -> bool {
        self.chapter == other.chapter
    }
}