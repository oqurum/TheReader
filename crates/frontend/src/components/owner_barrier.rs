use std::rc::Rc;

use yew::prelude::*;

use crate::AppState;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub children: Children,
}

#[function_component(OwnerBarrier)]
pub fn _owner_barrier(props: &Props) -> Html {
    let state = use_context::<Rc<AppState>>().unwrap();

    let Some(member) = state.member.as_ref() else {
        return html! {};
    };

    if member.permissions.is_owner() {
        html! {
            for props.children.iter()
        }
    } else {
        html! {}
    }
}
