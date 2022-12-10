use yew::{html, Html};
use yew_router::Routable;

mod admin;
mod member;
mod sidebar;

pub use admin::*;
pub use sidebar::SettingsSidebar;

use crate::get_member_self;

// TODO: Remove once I add General Routes
#[allow(clippy::enum_variant_names)]
#[derive(Routable, PartialEq, Eq, Clone, Copy, Debug)]
pub enum SettingsRoute {
    // Admin Routes
    #[at("/settings/libraries")]
    AdminLibraries,

    #[at("/settings/members")]
    AdminMembers,

    #[at("/settings/myserver")]
    AdminMyServer,

    #[at("/settings/tasks")]
    AdminTasks,

    // General Routes
    #[at("/settings/general")]
    General,
}

pub fn switch_settings(route: &SettingsRoute) -> Html {
    let member = get_member_self().unwrap();

    // TODO: Move once I have general settings.
    if !member.permissions.is_owner() {
        return html_container("No Permissions");
    }

    match route {
        SettingsRoute::AdminLibraries => {
            html! { <AdminLibrariesPage /> }
        }

        SettingsRoute::AdminMembers => {
            html! { <AdminMembersPage /> }
        }

        SettingsRoute::AdminMyServer => {
            html! { <AdminMyServerPage /> }
        }

        SettingsRoute::AdminTasks => {
            html! { <AdminTaskPage /> }
        }

        SettingsRoute::General => html_container("Not Implemented"),
    }
}

fn html_container(value: &'static str) -> Html {
    html! {
        <div class="outer-view-container">
            <SettingsSidebar />
            <div class="view-container">
                <h1>{ value }</h1>
            </div>
        </div>
    }
}
