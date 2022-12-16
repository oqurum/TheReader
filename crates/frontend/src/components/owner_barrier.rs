use yew::prelude::*;

use crate::get_member_self;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub children: Children,
}

#[function_component(OwnerBarrier)]
pub fn _owner_barrier(props: &Props) -> Html {
    let Some(member) = get_member_self() else {
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
