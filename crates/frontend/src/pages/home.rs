use yew::prelude::*;

use crate::components::Sidebar;

pub struct HomePage;

impl Component for HomePage {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <Sidebar />
                <div class="view-container">
                    //
                </div>
            </div>
        }
    }
}