use wasm_bindgen::{UnwrapThrowExt, JsCast, prelude::Closure};
use web_sys::HtmlIFrameElement;
use yew::Context;

use crate::{components::{Reader, reader::{section::SectionContents, ReaderMsg}}, util::ElementEvent};

use super::PAGE_DISPLAYS;



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
