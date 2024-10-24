use std::{cell::Cell, rc::Rc};

use chrono::Utc;
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::{Document, HtmlElement, HtmlIFrameElement, MouseEvent, WheelEvent};
use yew::Context;

use crate::{
    components::{
        reader::{section::SectionContents, DragType, OverlayEvent, ReaderMsg},
        Reader,
    },
    util::ElementEvent,
};

use super::PAGE_DISPLAYS;

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

                link.send_message(ReaderMsg::HandleViewOverlay((
                    None,
                    OverlayEvent::Release {
                        instant: Some(duration),

                        x: e.x(),
                        y: e.y(),

                        width: 0,
                        height: 0,
                    },
                )))
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

    pub fn set_chapter(&mut self, index: usize, section: &mut SectionContents) -> bool {
        let Some(chapter) = section.get_chapters().iter().find(|v| v.value == index) else {
            return false;
        };

        let Some(element) = section.find_section_start(chapter.value) else {
            return false;
        };

        element.scroll_into_view();

        true
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

        // TODO: Utilize section.page_offset - based off height of device.

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

impl Clone for ScrollDisplay {
    fn clone(&self) -> Self {
        Self {
            class_name: self.class_name,

            _events: Vec::new(),
        }
    }
}
