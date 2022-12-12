use yew::{html::Scope, prelude::*};
use yew_router::{prelude::Link, scope_ext::RouterScopeExt};

use super::SettingsRoute;

const ADMIN_LOCATIONS: [(&str, SettingsRoute); 4] = [
    ("Tasks", SettingsRoute::AdminTasks),
    ("Members", SettingsRoute::AdminMembers),
    ("My Server", SettingsRoute::AdminMyServer),
    ("Libraries", SettingsRoute::AdminLibraries),
];

pub struct SettingsSidebarContents;

impl Component for SettingsSidebarContents {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        SettingsSidebarContents
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="sidebar-item">
                    <h3>
                        { "Admin" }
                    </h3>
                </div>

                <ul class="nav nav-pills flex-column mb-auto">
                    { for ADMIN_LOCATIONS.iter().map(|&(title, route)| Self::render_sidebar_item(title, route, ctx.link())) }
                </ul>
            </>
        }
    }
}

impl SettingsSidebarContents {
    fn render_sidebar_item(title: &'static str, route: SettingsRoute, scope: &Scope<Self>) -> Html {
        let cr = scope.route::<SettingsRoute>().unwrap();

        html! {
            <li class="nav-item">
                <Link<SettingsRoute> to={route} classes={ classes!("nav-link", (cr == route).then_some("active")) }>
                    <span class="title">{ title }</span>
                </Link<SettingsRoute>>
            </li>
        }
    }
}
