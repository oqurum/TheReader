use std::{rc::Rc, cell::Cell};

use chrono::Utc;
use gloo_timers::callback::Timeout;
use wasm_bindgen::{UnwrapThrowExt, JsCast, prelude::Closure, JsValue};
use web_sys::{HtmlIFrameElement, HtmlElement, WheelEvent, Document, EventTarget};
use yew::Context;

use super::{CachedPage, Reader, ReaderMsg, DragType, OverlayEvent};

pub struct ElementEvent {
    element: EventTarget,
    function: Box<dyn AsRef<JsValue>>,

    destructor: Option<Box<dyn FnOnce(&EventTarget, &js_sys::Function) -> std::result::Result<(), JsValue>>>,
}

impl ElementEvent {
    pub fn link<C: AsRef<JsValue> + 'static, F: FnOnce(&EventTarget, &js_sys::Function) -> std::result::Result<(), JsValue>>(
        element: EventTarget,
        function: C,
        creator: F,
        destructor: Box<dyn FnOnce(&EventTarget, &js_sys::Function) -> std::result::Result<(), JsValue>>,
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


    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>)  {
        match self {
            SectionDisplay::Single(v) |
            SectionDisplay::Double(v) => v.add_to_iframe(iframe, ctx),
            SectionDisplay::Scroll(v) => v.add_to_iframe(iframe, ctx),
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

            _ => unreachable!()
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
        matches!((self, other), (Self::Single(_), Self::Single(_)) | (Self::Double(_), Self::Double(_)) | (Self::Scroll(_), Self::Scroll(_)))
    }
}




// Page Display

#[derive(Debug, Clone)]
pub struct PageDisplay {
    /// Total pages Being displayed at once.
    count: usize,

    cached_pages: Vec<CachedPage>,

    viewing_page: usize,
}

impl PageDisplay {
    pub fn new(count: usize) -> Self {
        Self {
            count,

            cached_pages: Vec::new(),

            viewing_page: 0
        }
    }

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>)  {
        //
    }

    pub fn page_count(&self) -> usize {
        self.cached_pages.len()
    }

    pub fn set_page(&mut self, index: usize, iframe: &HtmlIFrameElement) -> bool {
        if index >= self.cached_pages.len() {
            return false;
        }

        self.viewing_page = index;

        let body = iframe.content_document().unwrap_throw()
            .body().unwrap_throw();

        body.style().set_property("transition", "left 0.5s ease 0s").unwrap_throw();
        body.style().set_property("left", &format!("calc(-{}% - {}px)", 100 * self.viewing_page, self.viewing_page * 10)).unwrap_throw();

        true
    }

    pub fn next_page(&mut self, iframe: &HtmlIFrameElement) -> bool {
        if self.viewing_page + 1 < self.page_count() {
            self.set_page(self.viewing_page + 1, iframe)
        } else {
            false
        }
    }

    pub fn previous_page(&mut self, iframe: &HtmlIFrameElement) -> bool {
        if self.viewing_page != 0 {
            self.set_page(self.viewing_page - 1, iframe)
        } else {
            false
        }
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

    pub fn add_to_iframe(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Reader>)  {
        { // Scroll Display - Used to handle section changing with the scroll wheel.
            let body = iframe.content_document().unwrap();
            let link = ctx.link().clone();

            let function = Closure::wrap(Box::new(move |e: WheelEvent| {
                let el: Document = e.current_target().unwrap_throw().unchecked_into();
                let scrolling_element = el.scrolling_element().unwrap_throw();

                let delta = e.delta_y();
                let is_scrolling_down = delta.is_sign_positive();

                if !is_scrolling_down && scrolling_element.scroll_top() == 0 {
                    // At the start
                    link.send_message(ReaderMsg::HandleScrollChangePage(DragType::Up(delta.abs() as usize)));
                } else if is_scrolling_down && scrolling_element.scroll_top() + scrolling_element.client_height() >= scrolling_element.scroll_height() {
                    // At the end
                    link.send_message(ReaderMsg::HandleScrollChangePage(DragType::Down(delta.abs() as usize)));
                }
            }) as Box<dyn FnMut(WheelEvent)>);


            self._events.push(ElementEvent::link(
                body.unchecked_into(),
                function,
                |element, func| element.add_event_listener_with_callback("wheel", func),
                Box::new(|element, func| element.remove_event_listener_with_callback("wheel", func))
            ));
        }

        { // Scroll Display - On click
            let link = ctx.link().clone();
            let press_duration = Rc::new(Cell::new(Utc::now()));
            let body = iframe.content_document().unwrap_throw();

            let press_duration2 = press_duration.clone();
            let function_md = Closure::wrap(Box::new(move || {
                press_duration2.set(Utc::now());
            }) as Box<dyn FnMut()>);

            let function_mu = Closure::wrap(Box::new(move || {
                let duration = Utc::now().signed_duration_since(press_duration.get());

                link.send_message(ReaderMsg::HandleViewOverlay(OverlayEvent {
                    type_of: DragType::None,
                    dragging: false,
                    instant: Some(duration),
                }))
            }) as Box<dyn FnMut()>);


            self._events.push(ElementEvent::link(
                body.clone().unchecked_into(),
                function_md,
                |element, func| element.add_event_listener_with_callback("mousedown", func),
                Box::new(|element, func| element.remove_event_listener_with_callback("mousedown", func))
            ));

            self._events.push(ElementEvent::link(
                body.unchecked_into(),
                function_mu,
                |element, func| element.add_event_listener_with_callback("mouseup", func),
                Box::new(|element, func| element.remove_event_listener_with_callback("mouseup", func))
            ));
        }
    }

    pub fn page_count(&self) -> usize {
        1
    }

    /// Will only scroll to the start of the section
    pub fn set_page(&mut self, _index: usize, iframe: &HtmlIFrameElement) -> bool {
        let el = iframe.content_document().unwrap_throw()
            .scrolling_element().unwrap_throw();

        el.scroll_with_x_and_y(0.0, 0.0);

        true
    }

    pub fn next_page(&mut self) -> bool {
        false
    }

    pub fn previous_page(&mut self) -> bool {
        false
    }

    pub fn on_stop_viewing(&self, iframe: &HtmlIFrameElement) {
        let el: HtmlElement = iframe.content_document().unwrap_throw()
            .scrolling_element().unwrap_throw()
            .unchecked_into();

        el.style().set_property("overflow", "hidden").unwrap_throw();
    }

    pub fn on_start_viewing(&self, iframe: &HtmlIFrameElement) {
        let el: HtmlElement = iframe.content_document().unwrap_throw()
            .scrolling_element().unwrap_throw()
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