// TODO: Make component Sidebar modular and remove this one?

use yew::{html::Scope, prelude::*};
use yew_router::{prelude::Link, scope_ext::RouterScopeExt};

use super::SettingsRoute;

const ADMIN_LOCATIONS: [(&str, SettingsRoute); 4] = [
    ("Tasks", SettingsRoute::AdminTasks),
    ("Members", SettingsRoute::AdminMembers),
    ("My Server", SettingsRoute::AdminMyServer),
    ("Libraries", SettingsRoute::AdminLibraries),
];

pub struct SettingsSidebar;

impl Component for SettingsSidebar {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        SettingsSidebar
    }


    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="sidebar-container">
                { for ADMIN_LOCATIONS.iter().map(|&(title, route)| Self::render_sidebar_item(title, route, ctx.link())) }
            </div>
        }
    }
}

impl SettingsSidebar {
    fn render_sidebar_item(title: &'static str, route: SettingsRoute, scope: &Scope<Self>) -> Html {
        let cr = scope.route::<SettingsRoute>().unwrap();

        html! {
            <Link<SettingsRoute> to={route} classes={ classes!("sidebar-item", (cr == route).then_some("active")) }>
                <span class="title">{ title }</span>
            </Link<SettingsRoute>>
        }
    }
}
