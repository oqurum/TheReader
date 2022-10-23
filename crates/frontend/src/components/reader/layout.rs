use std::{cell::Cell, rc::Rc};

use chrono::Utc;
use gloo_timers::callback::Timeout;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue, UnwrapThrowExt};
use web_sys::{Document, EventTarget, HtmlElement, HtmlIFrameElement, MouseEvent, WheelEvent};
use yew::Context;

use super::{section::SectionContents, DragType, OverlayEvent, Reader, ReaderMsg};

type Destructor =
    Box<dyn FnOnce(&EventTarget, &js_sys::Function) -> std::result::Result<(), JsValue>>;

pub struct ElementEvent {
    element: EventTarget,
    function: Box<dyn AsRef<JsValue>>,

    destructor: Option<Destructor>,
}

impl ElementEvent {
    pub fn link<
        C: AsRef<JsValue> + 'static,
        F: FnOnce(&EventTarget, &js_sys::Function) -> std::result::Result<(), JsValue>,
    >(
        element: EventTarget,
        function: C,
        creator: F,
        destructor: Destructor,
    ) -> Self {
        let this = Self {
            element,
            function: Box::new(function),
            destructor: Some(destructor),
        };

        creator(&this.element, (*this.function).as_ref().unchecked_ref()).unwrap_throw();

        this
    }
}

impl Drop for ElementEvent {
    fn drop(&mut self) {
        if let Some(dest) = self.destructor.take() {
            dest(&self.element, (*self.function).as_ref().unchecked_ref()).unwrap_throw();
        }
    }
}

#[derive(Debug, Clone)]
pub enum SectionDisplay {
    Single(PageDisplay),
    Double(PageDisplay),
    Scroll(ScrollDisplay),
}

impl SectionDisplay {
    pub fn new_single() -> Self {
        Self::Single(PageDisplay::new(1))
    }

    pub fn new_double() -> Self {
        Self::Double(PageDisplay::new(2))
    }

    pub fn new_scroll() -> Self {
        Self::Scroll(ScrollDisplay::new())
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.add_to_iframe(iframe, ctx),
            SectionDisplay::Scroll(v) => v.add_to_iframe(iframe, ctx),
        }
    }

    pub fn set_page(&mut self, index: usize, section: &mut SectionContents) -> bool {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.set_page(index, section),
            SectionDisplay::Scroll(v) => v.set_page(index, section),
        }
    }

    pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.next_page(section),
            SectionDisplay::Scroll(v) => v.next_page(),
        }
    }

    pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.previous_page(section),
            SectionDisplay::Scroll(v) => v.previous_page(),
        }
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        match self {
            SectionDisplay::Single(v) | SectionDisplay::Double(v) => v.set_last_page(section),
            SectionDisplay::Scroll(v) => v.set_last_page(section),
        }
    }

    pub fn on_start_viewing(&self, section: &SectionContents) {
        match self {
            SectionDisplay::Single(_) | SectionDisplay::Double(_) => (),
            SectionDisplay::Scroll(v) => v.on_start_viewing(section),
        }
    }

    pub fn on_stop_viewing(&self, section: &SectionContents) {
        match self {
            SectionDisplay::Single(_) | SectionDisplay::Double(_) => (),
            SectionDisplay::Scroll(v) => v.on_stop_viewing(section),
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

    pub fn as_u8(&self) -> u8 {
        match self {
            SectionDisplay::Single(_) => 0,
            SectionDisplay::Double(_) => 1,
            SectionDisplay::Scroll(_) => 2,
        }
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

    _events: Vec<ElementEvent>,
}

impl PageDisplay {
    pub fn new(count: usize) -> Self {
        Self {
            count,

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
        let link = ctx.link().clone();

        let function = Closure::wrap(
            Box::new(move || link.send_message(ReaderMsg::UploadProgress)) as Box<dyn FnMut()>,
        );

        self._events.push(ElementEvent::link(
            body.unchecked_into(),
            function,
            |element, func| element.add_event_listener_with_callback("transitionend", func),
            Box::new(|element, func| {
                element.remove_event_listener_with_callback("transitionend", func)
            }),
        ));
    }

    pub fn set_page(&mut self, index: usize, section: &mut SectionContents) -> bool {
        if index >= section.page_count() {
            return false;
        }

        section.viewing_page = index;

        let body = section
            .get_iframe()
            .content_document()
            .unwrap_throw()
            .body()
            .unwrap_throw();

        body.style()
            .set_property("transition", "left 0.5s ease 0s")
            .unwrap_throw();
        body.style()
            .set_property(
                "left",
                &format!(
                    "calc(-{}% - {}px)",
                    100 * section.viewing_page,
                    section.viewing_page * 10
                ),
            )
            .unwrap_throw();

        true
    }

    pub fn next_page(&mut self, section: &mut SectionContents) -> bool {
        if section.viewing_page + 1 < section.page_count() {
            self.set_page(section.viewing_page + 1, section)
        } else {
            false
        }
    }

    pub fn previous_page(&mut self, section: &mut SectionContents) -> bool {
        if section.viewing_page != 0 {
            self.set_page(section.viewing_page - 1, section)
        } else {
            false
        }
    }

    pub fn set_last_page(&mut self, section: &mut SectionContents) {
        self.set_page(section.page_count().saturating_sub(1), section);
    }
}

// TODO: Remove
impl Clone for PageDisplay {
    fn clone(&self) -> Self {
        Self {
            count: self.count,
            _events: Vec::new(),
        }
    }
}

impl std::fmt::Debug for PageDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageDisplay")
            .field("count", &self.count)
            .finish()
    }
}

// Scroll Display

pub struct ScrollDisplay {
    change_page_timeout: Option<Timeout>,

    _events: Vec<ElementEvent>,
}

impl ScrollDisplay {
    pub fn new() -> Self {
        Self {
            change_page_timeout: None,

            _events: Vec::new(),
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>) {
        let body = iframe.content_document().unwrap();

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

                link.send_message(ReaderMsg::HandleViewOverlay(OverlayEvent::Swipe {
                    type_of: DragType::None,
                    dragging: false,
                    instant: Some(duration),
                    coords_start: (e.x(), e.y()),
                    coords_end: (e.x(), e.y()),
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

// TODO: Remove
impl Clone for ScrollDisplay {
    fn clone(&self) -> Self {
        Self {
            change_page_timeout: None,
            _events: Vec::new(),
        }
    }
}

impl std::fmt::Debug for ScrollDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollDisplay")
            .field("change_page_timeout", &self.change_page_timeout)
            .finish()
    }
}
