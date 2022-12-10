use yew::prelude::*;

use crate::pages::settings::SettingsSidebar;

pub struct AdminTaskPage;

impl Component for AdminTaskPage {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        // let member = get_member_self().unwrap();

        html! {
            <div class="outer-view-container">
                <SettingsSidebar />
                <div class="view-container">
                    <h2>{ "Tasks" }</h2>

                    <br />

                    <h4>{ "Viewing not implemented" }</h4>
                </div>
            </div>
        }
    }
}
