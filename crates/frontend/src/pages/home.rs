use books_common::{api, LibraryColl};
use yew::{prelude::*, html::Scope};
use yew_router::prelude::Link;

use crate::{Route, request};

pub enum Msg {
	// Results
	LibraryListResults(api::GetLibrariesResponse)
}

pub struct HomePage {
	library_items: Option<Vec<LibraryColl>>
}

impl Component for HomePage {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			library_items: None
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::LibraryListResults(resp) => {
				self.library_items = Some(resp.items);
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(items) = self.library_items.as_deref() {
			html! {
				<div class="home-view-container">
					<div class="sidebar">
						{ for items.iter().map(|item| Self::render_sidebar_library_item(item, ctx.link())) }
					</div>
					<div class="main-content-view">
						//
					</div>
				</div>
			}
		} else {
			html! {
				<h1>{ "Loading..." }</h1>
			}
		}
	}

	fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
		if first_render {
			ctx.link()
			.send_future(async move {
				Msg::LibraryListResults(request::get_libraries().await)
			});
		}
	}
}

impl HomePage {
	fn render_sidebar_library_item(item: &LibraryColl, _scope: &Scope<Self>) -> Html {
		html! {
			<Link<Route> to={Route::ViewLibrary { library_id: item.id }} classes={ classes!("sidebar-item", "library") }>
				{ item.name.clone() }
			</Link<Route>>
		}
	}
}