use std::{cell::RefCell, rc::Rc};

use chrono::{DateTime, Utc};
use common::api::{ApiErrorResponse, ErrorCodeResponse};
use common_local::filter::FilterContainer;
use gloo_utils::window;
use js_sys::Function;
use lazy_static::lazy_static;
use regex::Regex;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{EventTarget, MouseEvent};
use yew::{html::Scope, Callback, Component};

use crate::Result;

lazy_static! {
    static ref MOBILE_TABLET_CHECK: Regex = Regex::new(r"iP(ad|od|hone)|Tablet|Nexus|Mobile|IEMobile|MSIE [1-7]\.|Opera Mini|BB10|Symbian|webOS|Lenovo YT-|Android").unwrap_throw();
}

pub fn is_mobile_or_tablet() -> bool {
    MOBILE_TABLET_CHECK.is_match(&window().navigator().user_agent().unwrap_throw())
}

type Destructor = Box<dyn FnOnce(&EventTarget, &Function) -> std::result::Result<(), JsValue>>;

/// Allows for easier creation and destruction of event listener functions.
pub struct ElementEvent {
    element: EventTarget,
    function: Box<dyn AsRef<JsValue>>,

    destructor: Option<Destructor>,
}

impl ElementEvent {
    pub fn link<
        C: AsRef<JsValue> + 'static,
        F: FnOnce(&EventTarget, &Function) -> std::result::Result<(), JsValue>,
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

#[derive(Debug)]
pub struct UpdatableContext<V> {
    last_updated: DateTime<Utc>,
    content: Rc<RefCell<V>>,

    update_cb: Callback<()>,
}

impl<V: 'static> UpdatableContext<V> {
    pub fn new(value: V, update_cb: Callback<()>) -> Self {
        Self {
            update_cb,
            last_updated: Utc::now(),
            content: Rc::new(RefCell::new(value)),
        }
    }

    pub fn update<F: FnOnce(&mut V)>(&self, value: F) {
        let mut mutate = self.content.borrow_mut();

        value(&mut mutate);

        self.update_cb.emit(());
    }

    pub fn update_res<F: FnOnce(&mut V) -> Result<()>>(&self, value: F) -> Result<()> {
        let mut mutate = self.content.borrow_mut();

        value(&mut mutate)?;

        self.update_cb.emit(());

        Ok(())
    }

    pub fn update_if<F: FnOnce(&mut V) -> bool>(&self, value: F) -> bool {
        let mut mutate = self.content.borrow_mut();

        if value(&mut mutate) {
            self.update_cb.emit(());
            true
        } else {
            false
        }
    }

    pub fn reform<F: Fn(A, &mut V) + 'static, A>(&self, value: F) -> Callback<A> {
        let this = self.clone();

        Callback::from(move |e| {
            let mut mutate = this.content.borrow_mut();

            value(e, &mut mutate);

            this.update_cb.emit(());
        })
    }

    pub fn borrow(&self) -> std::cell::Ref<'_, V> {
        self.content.borrow()
    }

    pub fn set_updated(&mut self) {
        self.last_updated = Utc::now();
    }
}

impl<V> PartialEq for UpdatableContext<V> {
    fn eq(&self, other: &Self) -> bool {
        self.last_updated == other.last_updated
    }
}

impl<V> Clone for UpdatableContext<V> {
    fn clone(&self) -> Self {
        Self {
            last_updated: self.last_updated.clone(),
            content: self.content.clone(),
            update_cb: self.update_cb.clone(),
        }
    }
}

pub fn as_local_path_without_http(value: &str) -> String {
    let loc = window().location();

    let host = loc.host().unwrap_throw();

    format!(
        "{host}/{}",
        if let Some(v) = value.strip_prefix('/') {
            v
        } else {
            value
        }
    )
}

/// A Callback which calls "prevent_default" and "stop_propagation"
///
/// Also will prevent any more same events downstream from activating
pub fn on_click_prevdef_stopprop_cb<S: 'static, F: Fn(&Callback<S>, MouseEvent) + 'static>(
    cb: Callback<S>,
    func: F,
) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();

        func(&cb, e);
    })
}

/// A Callback which calls "prevent_default"
pub fn on_click_prevdef_cb<S: 'static, F: Fn(&Callback<S>, MouseEvent) + 'static>(
    cb: Callback<S>,
    func: F,
) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        e.prevent_default();

        func(&cb, e);
    })
}

/// A Callback which calls "prevent_default" and "stop_propagation"
///
/// Also will prevent any more same events downstream from activating
pub fn on_click_prevdef_stopprop_scope<S, F>(cb: Scope<S>, func: F) -> Callback<MouseEvent>
where
    S: Component,
    F: (Fn(MouseEvent) -> S::Message) + 'static,
{
    Callback::from(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();

        cb.send_message(func(e));
    })
}

/// A Callback which calls "prevent_default"
pub fn on_click_prevdef_scope<S, F>(cb: Scope<S>, func: F) -> Callback<MouseEvent>
where
    S: Component,
    F: (Fn(MouseEvent) -> S::Message) + 'static,
{
    Callback::from(move |e: MouseEvent| {
        e.prevent_default();

        cb.send_message(func(e));
    })
}

pub fn build_book_filter_query() -> FilterContainer {
    let q = gloo_utils::window().location().search().unwrap_throw();

    if q.is_empty() {
        FilterContainer::default()
    } else {
        match serde_qs::from_str(&q[1..]) {
            Ok(v) => v,
            Err(e) => {
                crate::display_error(ApiErrorResponse {
                    code: ErrorCodeResponse::Client,
                    description: e.to_string(),
                });

                FilterContainer::default()
            }
        }
    }
}
