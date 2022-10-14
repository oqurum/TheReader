use common::api::ApiErrorResponse;
use common_local::filter::FilterContainer;
use gloo_utils::window;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::MouseEvent;
use yew::{html::Scope, Callback, Component};

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
pub fn on_click_prevdef_stopprop_scope<S, F>(
    cb: Scope<S>,
    func: F,
) -> Callback<MouseEvent>
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
pub fn on_click_prevdef_scope<S, F>(
    cb: Scope<S>,
    func: F,
) -> Callback<MouseEvent>
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
                    description: e.to_string(),
                });

                FilterContainer::default()
            }
        }
    }
}