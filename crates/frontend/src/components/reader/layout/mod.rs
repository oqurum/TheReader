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


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMovement {
    LeftToRight,
    RightToLeft,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionDisplayType {
    Single,
    Double,
    Scroll,
    Image,
}


pub enum SectionDisplay {
    // Optimized Text Layouts
    Single(PageDisplay),
    Double(PageDisplay),
    // TODO: Paged Vertical
    // TODO: Rename to continuous vertical
    Scroll(ScrollDisplay),

    // Optimized Image Layouts
    Image(ImageDisplay),
}

impl SectionDisplay {
    pub fn new_single() -> Self {
        Self::Single(PageDisplay::new(1, "single-page"))
    }

    pub fn new_double() -> Self {
        Self::Double(PageDisplay::new(2, "double-page"))
    }

    pub fn new_scroll() -> Self {
        Self::Scroll(ScrollDisplay::new("scrolling-page"))
    }

    pub fn new_image(value: PageMovement) -> Self {
        Self::Image(ImageDisplay::new("image-page", value))
    }

    pub fn as_type(&self) -> SectionDisplayType {
        match self {
            SectionDisplay::Single(_) => SectionDisplayType::Single,
            SectionDisplay::Double(_) => SectionDisplayType::Double,
            SectionDisplay::Scroll(_) => SectionDisplayType::Scroll,
            SectionDisplay::Image(_) => SectionDisplayType::Image,
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.add_to_iframe(iframe, ctx),
            SectionDisplay::Scroll(v) => v.add_to_iframe(iframe, ctx),
            SectionDisplay::Image(v) => v.add_to_iframe(iframe, ctx),
        }
    }

    pub fn transitioning_page(&self, amount: isize, section: &SectionContents) {
        match self {
            SectionDisplay::Single(_) | SectionDisplay::Double(_) | SectionDisplay::Scroll(_) => {
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

            SectionDisplay::Image(v) => v.transitioning_page(amount, section),
        }
    }

    pub fn set_page(&mut self, index: usize, section: &mut SectionContents) -> bool {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.set_page(index, section),
            SectionDisplay::Scroll(v) => v.set_page(index, section),
            SectionDisplay::Image(v) => v.set_page(index, section),
        }
    }

    pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.next_page(section),
            SectionDisplay::Scroll(v) => v.next_page(),
            SectionDisplay::Image(v) => v.next_page(section),
        }
    }

    pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.previous_page(section),
            SectionDisplay::Scroll(v) => v.previous_page(),
            SectionDisplay::Image(v) => v.previous_page(section),
        }
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.set_last_page(section),
            SectionDisplay::Scroll(v) => v.set_last_page(section),
            SectionDisplay::Image(v) => v.set_last_page(section),
        }
    }

    pub fn on_start_viewing(&self, section: &SectionContents) {
        match self {
            SectionDisplay::Single(_) | SectionDisplay::Double(_) | SectionDisplay::Image(_) => (),
            SectionDisplay::Scroll(v) => v.on_start_viewing(section),
        }
    }

    pub fn on_stop_viewing(&self, section: &SectionContents) {
        match self {
            SectionDisplay::Single(_) | SectionDisplay::Double(_) | SectionDisplay::Image(_) => (),
            SectionDisplay::Scroll(v) => v.on_stop_viewing(section),
        }
    }

    pub fn get_movement(&self) -> PageMovement {
        match self {
            SectionDisplay::Single(_) | SectionDisplay::Double(_) | SectionDisplay::Scroll(_) => PageMovement::LeftToRight,
            SectionDisplay::Image(v) => v.movement,
        }
    }

    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_))
    }

    pub fn is_double(&self) -> bool {
        matches!(self, Self::Double(_))
    }

    pub fn is_scroll(&self) -> bool {
        matches!(self, Self::Scroll(_))
    }
}

impl From<u8> for SectionDisplay {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::new_single(),
            1 => Self::new_double(),
            2 => Self::new_scroll(),

            _ => unreachable!(),
        }
    }
}

impl Default for SectionDisplay {
    fn default() -> Self {
        Self::new_double()
    }
}

impl PartialEq for SectionDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.as_type() == other.as_type()
    }
}
