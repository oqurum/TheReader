use yew::{prelude::*, html::Scope};
use yew_router::prelude::*;

use components::NavbarModule;

mod util;
mod pages;
mod request;
mod components;

enum Msg {
	Load
}

struct Model {
	has_initial_loaded: bool,
}

impl Component for Model {
	type Message = Msg;
	type Properties = ();

	fn create(ctx: &Context<Self>) -> Self {
		ctx.link().send_message(Msg::Load);

		Self {
			has_initial_loaded: false,
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Load => {
				self.has_initial_loaded = true;
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if self.has_initial_loaded {
			let link = ctx.link().clone();

			html! {
				<BrowserRouter>
					<NavbarModule />
					<Switch<Route> render={Switch::render(move |r| switch(r, link.clone()))} />
				</BrowserRouter>
			}
		} else {
			html! {
				<h1>{ "Initiating..." }</h1>
			}
		}
	}
}

impl Model {
	//
}

#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
	#[at("/library/:library_id")]
	ViewLibrary { library_id: i64 },

	#[at("/read/:book_id")]
	ReadBook { book_id: usize },

	#[at("/options")]
	Options,

	#[at("/")]
	#[not_found]
	Dashboard
}


fn switch(route: &Route, _link: Scope<Model>) -> Html {
	log::info!("{:?}", route);
	match route.clone() {
		Route::ViewLibrary { library_id } => {
			html! { <pages::LibraryPage library_id={library_id}  /> }
		}

		Route::ReadBook { book_id } => {
			html! { <pages::ReadingBook id={book_id}  /> }
		}

		Route::Options => {
			html! { <pages::OptionsPage /> }
		}

		Route::Dashboard => {
			html! { <pages::HomePage /> }
		}
	}
}


fn main() {
	wasm_logger::init(wasm_logger::Config::default());

	yew::start_app::<Model>();
}