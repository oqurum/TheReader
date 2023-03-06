use std::{cell::Cell, rc::Rc};

use chrono::Utc;
use gloo_timers::callback::Timeout;
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::{Document, HtmlElement, HtmlIFrameElement, MouseEvent, WheelEvent};
use yew::Context;

use crate::util::ElementEvent;

use super::{section::SectionContents, DragType, OverlayEvent, Reader, ReaderMsg};

static PAGE_DISPLAYS: [&str; 3] = ["single-page", "double-page", "scrolling-page"];


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

    pub fn viewing_page(&self, section: &SectionContents) -> usize {
        section.page_offset // TODO: Remove.
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
        matches!(
            (self, other),
            (Self::Single(_), Self::Single(_))
                | (Self::Double(_), Self::Double(_))
                | (Self::Scroll(_), Self::Scroll(_))
        )
    }
}

// Page Display

pub struct PageDisplay {
    /// Total pages Being displayed at once.
    #[allow(dead_code)]
    count: usize,
    class_name: &'static str,

    _events: Vec<ElementEvent>,
}

impl PageDisplay {
    pub fn new(count: usize, class_name: &'static str) -> Self {
        Self {
            count,
            class_name,

            _events: Vec::new(),
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        // Page changes use a transition. After the transition ends we'll upload the progress.
        // Fixes the issue of js_get_current_by_pos being incorrect.

        let body = iframe
            .content_document()
            .unwrap_throw()
            .body()
            .unwrap_throw();

        PAGE_DISPLAYS.into_iter().for_each(|v| {
            let _ = body.class_list().remove_1(v);
        });

        body.class_list().add_1(self.class_name).unwrap_throw();

        let link = ctx.link().clone();

        let function =
            Closure::wrap(
                Box::new(move || link.send_message(ReaderMsg::PageTransitionEnd))
                    as Box<dyn FnMut()>,
            );

        self._events.push(ElementEvent::link(
            body.clone().unchecked_into(),
            function,
            |element, func| element.add_event_listener_with_callback("transitionend", func),
            Box::new(|element, func| {
                element.remove_event_listener_with_callback("transitionend", func)
            }),
        ));

        let link = ctx.link().clone();

        let function =
            Closure::wrap(
                Box::new(move || link.send_message(ReaderMsg::PageTransitionStart))
                    as Box<dyn FnMut()>,
            );

        self._events.push(ElementEvent::link(
            body.unchecked_into(),
            function,
            |element, func| element.add_event_listener_with_callback("transitionstart", func),
            Box::new(|element, func| {
                element.remove_event_listener_with_callback("transitionstart", func)
            }),
        ));
    }

    pub fn set_page(&mut self, index: usize, section: &mut SectionContents) -> bool {
        if index >= section.page_count() {
            return false;
        }

        section.page_offset = index;

        let body = section.get_iframe_body().unwrap_throw();

        body.style()
            .set_property("transition", "left 0.5s ease 0s")
            .unwrap_throw();
        body.style()
            .set_property(
                "left",
                &format!(
                    "calc(-{}% - {}px)",
                    100 * section.page_offset,
                    section.page_offset * 10
                ),
            )
            .unwrap_throw();

        true
    }

    pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
        if section.page_offset + 1 < section.page_count() {
            self.set_page(section.page_offset + 1, section)
        } else {
            false
        }
    }

    pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
        if section.page_offset != 0 {
            self.set_page(section.page_offset - 1, section)
        } else {
            false
        }
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        self.set_page(section.page_count().saturating_sub(1), section);
    }
}


// Scroll Display

pub struct ScrollDisplay {
    class_name: &'static str,

    _events: Vec<ElementEvent>,
}

impl ScrollDisplay {
    pub fn new(class_name: &'static str) -> Self {
        Self {
            class_name,

            _events: Vec::new(),
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        // TODO: name is `body` but  we're in the Document.
        let body = iframe.content_document().unwrap();

        {
            let body = body.body().unwrap_throw();
            PAGE_DISPLAYS.into_iter().for_each(|v| {
                let _ = body.class_list().remove_1(v);
            });
            body.class_list().add_1(self.class_name).unwrap_throw();
        }

        {
            // Scroll Display - Used to handle section changing with the scroll wheel.
            let link = ctx.link().clone();

            let function = Closure::wrap(Box::new(move |e: WheelEvent| {
                let el: Document = e.current_target().unwrap_throw().unchecked_into();
                let scrolling_element = el.scrolling_element().unwrap_throw();

                let delta = e.delta_y();
                let is_scrolling_down = delta.is_sign_positive();

                if !is_scrolling_down && scrolling_element.scroll_top() == 0 {
                    // At the start
                    link.send_message(ReaderMsg::HandleScrollChangePage(DragType::Up(
                        delta.abs() as usize
                    )));
                } else if is_scrolling_down
                    && scrolling_element.scroll_top() + scrolling_element.client_height()
                        >= scrolling_element.scroll_height()
                {
                    // At the end
                    link.send_message(ReaderMsg::HandleScrollChangePage(DragType::Down(
                        delta.abs() as usize,
                    )));
                }
            }) as Box<dyn FnMut(WheelEvent)>);

            self._events.push(ElementEvent::link(
                body.clone().unchecked_into(),
                function,
                |element, func| element.add_event_listener_with_callback("wheel", func),
                Box::new(|element, func| {
                    element.remove_event_listener_with_callback("wheel", func)
                }),
            ));
        }

        {
            // Scroll Display - On click
            let link = ctx.link().clone();
            let press_duration = Rc::new(Cell::new(Utc::now()));

            let press_duration2 = press_duration.clone();
            let function_md = Closure::wrap(Box::new(move || {
                press_duration2.set(Utc::now());
            }) as Box<dyn FnMut()>);

            let function_mu = Closure::wrap(Box::new(move |e: MouseEvent| {
                let duration = Utc::now().signed_duration_since(press_duration.get());

                link.send_message(ReaderMsg::HandleViewOverlay(OverlayEvent::Release {
                    instant: Some(duration),

                    x: e.x(),
                    y: e.y(),

                    width: 0,
                    height: 0,
                }))
            }) as Box<dyn FnMut(MouseEvent)>);

            self._events.push(ElementEvent::link(
                body.clone().unchecked_into(),
                function_md,
                |element, func| element.add_event_listener_with_callback("mousedown", func),
                Box::new(|element, func| {
                    element.remove_event_listener_with_callback("mousedown", func)
                }),
            ));

            self._events.push(ElementEvent::link(
                body.unchecked_into(),
                function_mu,
                |element, func| element.add_event_listener_with_callback("mouseup", func),
                Box::new(|element, func| {
                    element.remove_event_listener_with_callback("mouseup", func)
                }),
            ));
        }
    }

    /// Will only scroll to the start of the section
    pub fn set_page(&mut self, _index: usize, section: &mut SectionContents) -> bool {
        let el = section
            .get_iframe()
            .content_document()
            .unwrap_throw()
            .scrolling_element()
            .unwrap_throw();

        el.scroll_with_x_and_y(0.0, 0.0);

        true
    }

    pub fn next_page(&mut self) -> bool {
        false
    }

    pub fn previous_page(&mut self) -> bool {
        false
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        let el: HtmlElement = section
            .get_iframe()
            .content_document()
            .unwrap_throw()
            .scrolling_element()
            .unwrap_throw()
            .unchecked_into();

        el.scroll_with_x_and_y(0.0, el.scroll_height() as f64);
    }

    pub fn on_stop_viewing(&self, section: &SectionContents) {
        let el: HtmlElement = section
            .get_iframe()
            .content_document()
            .unwrap_throw()
            .scrolling_element()
            .unwrap_throw()
            .unchecked_into();

        el.style().set_property("overflow", "hidden").unwrap_throw();
    }

    pub fn on_start_viewing(&self, section: &SectionContents) {
        let el: HtmlElement = section
            .get_iframe()
            .content_document()
            .unwrap_throw()
            .scrolling_element()
            .unwrap_throw()
            .unchecked_into();

        el.style().remove_property("overflow").unwrap_throw();
    }
}


pub struct ImageDisplay {
    class_name: &'static str,
    _events: Vec<ElementEvent>,
    movement: PageMovement,
}

impl ImageDisplay {
    pub fn new(class_name: &'static str, movement: PageMovement) -> Self {
        Self {
            class_name,
            movement,

            _events: Vec::new(),
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        // Page changes use a transition. After the transition ends we'll upload the progress.
        // Fixes the issue of js_get_current_by_pos being incorrect.

        let body = iframe
            .content_document()
            .unwrap_throw()
            .body()
            .unwrap_throw();

        PAGE_DISPLAYS.into_iter().for_each(|v| {
            let _ = body.class_list().remove_1(v);
        });

        body.class_list().add_2(
            self.class_name,
            if self.movement == PageMovement::LeftToRight {
                "move-left-to-right"
            } else {
                "move-right-to-left"
            },
        ).unwrap_throw();

        let link = ctx.link().clone();

        let function =
            Closure::wrap(
                Box::new(move || link.send_message(ReaderMsg::PageTransitionEnd))
                    as Box<dyn FnMut()>,
            );

        self._events.push(ElementEvent::link(
            body.clone().unchecked_into(),
            function,
            |element, func| element.add_event_listener_with_callback("transitionend", func),
            Box::new(|element, func| {
                element.remove_event_listener_with_callback("transitionend", func)
            }),
        ));

        let link = ctx.link().clone();

        let function =
            Closure::wrap(
                Box::new(move || link.send_message(ReaderMsg::PageTransitionStart))
                    as Box<dyn FnMut()>,
            );

        self._events.push(ElementEvent::link(
            body.unchecked_into(),
            function,
            |element, func| element.add_event_listener_with_callback("transitionstart", func),
            Box::new(|element, func| {
                element.remove_event_listener_with_callback("transitionstart", func)
            }),
        ));
    }

    pub fn transitioning_page(&self, amount: isize, section: &SectionContents) {
        let body = section.get_iframe_body().unwrap_throw();

        let page = self.viewing_page(section);

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
                .set_property("transition", "all 0.5s ease 0s")
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

    pub fn set_page(&mut self, index: usize, section: &mut SectionContents) -> bool {
        if index >= section.page_count() {
            return false;
        }

        section.page_offset = index;

        let actual_offset = self.viewing_page(section);

        let body = section.get_iframe_body().unwrap_throw();

        // TODO: Fix the margin offset which is applied (the X * 10).

        if self.movement == PageMovement::RightToLeft {
            let count = section.page_count();

            body.style()
                .set_property(
                    "width",
                    &format!(
                        "calc({}% + {}px)",
                        100 * count,
                        actual_offset * 10
                    )
                )
                .unwrap_throw();
        }

        body.style()
            .set_property("transition", "all 0.5s ease 0s")
            .unwrap_throw();
        body.style()
            .set_property(
                "left",
                &format!(
                    "calc(-{}% - {}px)",
                    100 * actual_offset,
                    actual_offset * 10
                ),
            )
            .unwrap_throw();

        true
    }

    // pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
    //     if section.page_offset + 1 < section.page_count() {
    //         self.set_page(section.page_offset + 1, section)
    //     } else {
    //         false
    //     }
    // }

    // pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
    //     if section.page_offset != 0 {
    //         self.set_page(section.page_offset - 1, section)
    //     } else {
    //         false
    //     }
    // }

    pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
        match self.movement {
            // Forwards
            PageMovement::LeftToRight if section.page_offset + 1 < section.page_count() => self.set_page(section.page_offset + 1, section),
            // Backwards
            PageMovement::RightToLeft if section.page_offset != 0 => self.set_page(section.page_offset - 1, section),
            _ => false,
        }
    }

    pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
        match self.movement {
            // Backwards
            PageMovement::LeftToRight if section.page_offset != 0 => self.set_page(section.page_offset - 1, section),
            // Forwards
            PageMovement::RightToLeft if section.page_offset + 1 < section.page_count() => self.set_page(section.page_offset + 1, section),
            _ => false,
        }
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        self.set_page(section.page_count().saturating_sub(1), section);
    }

    pub fn viewing_page(&self, section: &SectionContents) -> usize {
        match self.movement {
            PageMovement::LeftToRight => section.page_offset,
            PageMovement::RightToLeft => (section.page_count() - section.page_offset).saturating_sub(1),
        }
    }
}
