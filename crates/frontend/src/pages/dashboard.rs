use books_common::{MediaItem, api};
use yew::{prelude::*, html::Scope};
use yew_router::prelude::Link;

use crate::{Route, request};

pub enum Msg {
	MediaListResults(api::GetBookListResponse)
}

pub struct DashboardPage {
	media_items: Option<Vec<MediaItem>>,
	total_media_count: i64
}

impl Component for DashboardPage {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			media_items: None,
			total_media_count: 0
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::MediaListResults(mut resp) => {
				self.total_media_count = resp.count;

				if let Some(items) = self.media_items.as_mut() {
					items.append(&mut resp.items);
				} else {
					self.media_items = Some(resp.items);
				}
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(items) = self.media_items.as_deref() {
			let link = ctx.link().clone();

			html! {
				<div class="library-list normal">
					{ for items.iter().map(|item| Self::render_media_item(item, &link)) }
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
			ctx.link().send_future(async {
				Msg::MediaListResults(request::get_books().await)
			});
		}
	}
}

impl DashboardPage {
	fn render_media_item(item: &MediaItem, scope: &Scope<Self>) -> Html {
		html! {
			<Link<Route> to={Route::ReadBook { book_id: item.id as usize }} classes={ classes!("library-item") }>
				<div class="poster">
					<img src={ item.icon_path.as_ref().cloned().unwrap_or_else(|| String::from("/images/missingthumbnail.jpg")) } />
				</div>
				<div class="info">
					<a class="author" title={ item.author.clone() }>{ item.author.clone() }</a>
					<a class="title" title={ item.title.clone() }>{ item.title.clone() }</a>
				</div>
			</Link<Route>>
		}
	}
}
