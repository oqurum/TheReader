use yew::prelude::*;
use yew_router::components::Link;

use crate::Route;

pub enum Msg {

}

pub struct NavbarModule {
	items: Vec<(Route, &'static str)>
}

impl Component for NavbarModule {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			items: vec![
				(Route::Dashboard, "Home"),
				(Route::Options, "Options")
			]
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
		true
	}

	fn view(&self, _ctx: &Context<Self>) -> Html {
		html! {
			<div class="navbar-module">
				{
					for self.items.iter().map(|item| html! {
						<Link<Route> to={item.0.clone()}>{ item.1 }</Link<Route>>
					})
				}
			</div>
		}
	}
}