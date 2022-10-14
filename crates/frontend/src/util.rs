use common::api::ApiErrorResponse;
use common_local::filter::FilterContainer;
use gloo_utils::window;
use serde::{Deserialize, Serialize};
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

pub fn update_query<F: FnOnce(&mut SearchQuery)>(value: F) {
    let mut query = SearchQuery::load();

    value(&mut query);

    query.save();
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub filters: FilterContainer,
}

impl SearchQuery {
    pub fn save(&self) {
        let s = serde_qs::to_string(self).unwrap_throw();

        gloo_utils::window()
            .location()
            .set_search(&s)
            .unwrap_throw();
    }

    pub fn load() -> Self {
        let q = gloo_utils::window().location().search().unwrap_throw();

        if q.is_empty() {
            Self::default()
        } else {
            match serde_qs::from_str(&q[1..]) {
                Ok(v) => v,
                Err(e) => {
                    crate::display_error(ApiErrorResponse {
                        description: e.to_string(),
                    });

                    Self::default()
                }
            }
        }
    }
}
