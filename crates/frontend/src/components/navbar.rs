use yew::prelude::*;
use yew_router::components::Link;

use crate::Route;

pub enum Msg {

}

pub struct NavbarModule {
	left_items: Vec<(Route, DisplayType)>,
	right_items: Vec<(Route, DisplayType)>,
}

impl Component for NavbarModule {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			left_items: vec![
				(Route::Dashboard, DisplayType::Icon("home", "Home")),
			],
			right_items: vec![
				(Route::Options, DisplayType::Icon("settings", "Settings")),
			],
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
		true
	}

	fn view(&self, _ctx: &Context<Self>) -> Html {
		html! {
			<div class="navbar-module">
				<div class="left-content">
				{
					for self.left_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1))
				}
				</div>
				<div class="center-content">
					<div class="search-bar">
						<input placeholder="Search" />
						<button>{ "Search" }</button>
					</div>
				</div>
				<div class="right-content">
				{
					for self.right_items.iter().map(|item| Self::render_item(item.0.clone(), &item.1))
				}
				</div>
			</div>
		}
	}
}

impl NavbarModule {
	fn render_item(route: Route, name: &DisplayType) -> Html {
		match name {
			DisplayType::Text(v) => html! {
				<Link<Route> to={route}>{ v }</Link<Route>>
			},
			DisplayType::Icon(icon, title) => html! {
				<Link<Route> to={route}>
					<span class="material-icons" title={ *title }>{ icon }</span>
				</Link<Route>>
			}
		}
	}
}

pub enum DisplayType {
	Text(&'static str),
	Icon(&'static str, &'static str),
}