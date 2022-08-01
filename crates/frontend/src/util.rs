use gloo_utils::window;
use web_sys::MouseEvent;
use yew::{html::Scope, Callback, Component};

pub fn as_local_path_without_http(value: &str) -> String {
    format!(
        "{}/{}",
        window().location().hostname().unwrap(),
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
pub fn on_click_prevdef_stopprop<S>(scope: &Scope<S>, msg: S::Message) -> Callback<MouseEvent>
    where S: Component,
        S::Message: Clone
{
    scope.callback(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        msg.clone()
    })
}

/// A Callback which calls "prevent_default"
pub fn on_click_prevdef<S>(scope: &Scope<S>, msg: S::Message) -> Callback<MouseEvent>
    where S: Component,
        S::Message: Clone
{
    scope.callback(move |e: MouseEvent| {
        e.prevent_default();
        msg.clone()
    })
}