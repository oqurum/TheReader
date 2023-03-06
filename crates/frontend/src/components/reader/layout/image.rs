use wasm_bindgen::{UnwrapThrowExt, prelude::Closure, JsCast};
use web_sys::HtmlIFrameElement;
use yew::Context;

use crate::{components::{Reader, reader::{ReaderMsg, section::SectionContents}}, util::ElementEvent};

use super::{PageMovement, PAGE_DISPLAYS};



pub struct ImageDisplay {
    class_name: &'static str,
    _events: Vec<ElementEvent>,
    pub(in super) movement: PageMovement,
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
