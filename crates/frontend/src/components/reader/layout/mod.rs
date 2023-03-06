//! Layouts for displaying the contents of an iframe.

use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlIFrameElement;
use yew::Context;

use super::{section::SectionContents, Reader};

static PAGE_DISPLAYS: [&str; 4] = ["single-page", "double-page", "scrolling-page", "image-page"];

mod image;
mod page;
mod scroll;

pub use image::*;
pub use page::*;
pub use scroll::*;

pub use common_local::reader::{LayoutType, PageMovement};


/// Describes how to display a section.
pub enum LayoutDisplay {
    // Optimized Text Layouts
    SinglePage(PageDisplay),
    DoublePage(PageDisplay),
    // TODO: Continuous vertical
    VerticalScroll(ScrollDisplay),

    // Optimized Image Layouts
    Image(ImageDisplay),
}

impl LayoutDisplay {
    pub fn new_single() -> Self {
        Self::SinglePage(PageDisplay::new(1, "single-page"))
    }

    pub fn new_double() -> Self {
        Self::DoublePage(PageDisplay::new(2, "double-page"))
    }

    pub fn new_scroll() -> Self {
        Self::VerticalScroll(ScrollDisplay::new("scrolling-page"))
    }

    pub fn new_image(value: PageMovement) -> Self {
        Self::Image(ImageDisplay::new("image-page", value))
    }

    pub fn as_type(&self) -> LayoutType {
        match self {
            LayoutDisplay::SinglePage(_) => LayoutType::Single,
            LayoutDisplay::DoublePage(_) => LayoutType::Double,
            LayoutDisplay::VerticalScroll(_) => LayoutType::Scroll,
            LayoutDisplay::Image(_) => LayoutType::Image,
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        match self {
            LayoutDisplay::SinglePage(v) | LayoutDisplay::DoublePage(v) => v.add_to_iframe(iframe, ctx),
            LayoutDisplay::VerticalScroll(v) => v.add_to_iframe(iframe, ctx),
            LayoutDisplay::Image(v) => v.add_to_iframe(iframe, ctx),
        }
    }

    pub fn transitioning_page(&self, amount: isize, section: &SectionContents) {
        match self {
            LayoutDisplay::SinglePage(_) | LayoutDisplay::DoublePage(_) | LayoutDisplay::VerticalScroll(_) => {
                let body = section.get_iframe_body().unwrap_throw();

                let page = section.page_offset;

                // Prevent empty pages when on the first or last page of a section.
                let amount = if (amount.is_positive() && page == 0)
                    || (amount.is_negative() && page == section.page_count().saturating_sub(1))
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
                            100 * page,
                            page as isize * 10 - amount
                        ),
                    )
                    .unwrap();
            }

            LayoutDisplay::Image(v) => v.transitioning_page(amount, section),
        }
    }

    pub fn set_page(&mut self, index: usize, section: &mut SectionContents) -> bool {
        match self {
            LayoutDisplay::SinglePage(v) | LayoutDisplay::DoublePage(v) => v.set_page(index, section),
            LayoutDisplay::VerticalScroll(v) => v.set_page(index, section),
            LayoutDisplay::Image(v) => v.set_page(index, section),
        }
    }

    pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
        match self {
            LayoutDisplay::SinglePage(v) | LayoutDisplay::DoublePage(v) => v.next_page(section),
            LayoutDisplay::VerticalScroll(v) => v.next_page(),
            LayoutDisplay::Image(v) => v.next_page(section),
        }
    }

    pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
        match self {
            LayoutDisplay::SinglePage(v) | LayoutDisplay::DoublePage(v) => v.previous_page(section),
            LayoutDisplay::VerticalScroll(v) => v.previous_page(),
            LayoutDisplay::Image(v) => v.previous_page(section),
        }
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        match self {
            LayoutDisplay::SinglePage(v) | LayoutDisplay::DoublePage(v) => v.set_last_page(section),
            LayoutDisplay::VerticalScroll(v) => v.set_last_page(section),
            LayoutDisplay::Image(v) => v.set_last_page(section),
        }
    }

    pub fn on_start_viewing(&self, section: &SectionContents) {
        match self {
            LayoutDisplay::SinglePage(_) | LayoutDisplay::DoublePage(_) | LayoutDisplay::Image(_) => (),
            LayoutDisplay::VerticalScroll(v) => v.on_start_viewing(section),
        }
    }

    pub fn on_stop_viewing(&self, section: &SectionContents) {
        match self {
            LayoutDisplay::SinglePage(_) | LayoutDisplay::DoublePage(_) | LayoutDisplay::Image(_) => (),
            LayoutDisplay::VerticalScroll(v) => v.on_stop_viewing(section),
        }
    }

    pub fn get_movement(&self) -> PageMovement {
        match self {
            LayoutDisplay::SinglePage(_) | LayoutDisplay::DoublePage(_) | LayoutDisplay::VerticalScroll(_) => PageMovement::LeftToRight,
            LayoutDisplay::Image(v) => v.movement,
        }
    }

    pub fn is_single(&self) -> bool {
        matches!(self, Self::SinglePage(_))
    }

    pub fn is_double(&self) -> bool {
        matches!(self, Self::DoublePage(_))
    }

    pub fn is_scroll(&self) -> bool {
        matches!(self, Self::VerticalScroll(_))
    }
}

impl From<LayoutType> for LayoutDisplay {
    fn from(value: LayoutType) -> Self {
        match value {
            LayoutType::Single => Self::new_single(),
            LayoutType::Double => Self::new_double(),
            LayoutType::Scroll => Self::new_scroll(),
            LayoutType::Image => unimplemented!(),
        }
    }
}

impl Default for LayoutDisplay {
    fn default() -> Self {
        Self::new_double()
    }
}

impl PartialEq for LayoutDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.as_type() == other.as_type()
    }
}
