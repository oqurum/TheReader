use books_common::{api::MediaViewResponse, util::file_size_bytes_to_readable_string};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{request, Route};


pub enum Msg {
	// Retrive
	RetrieveMediaView(MediaViewResponse),
}

#[derive(Properties, PartialEq)]
pub struct Property {
	pub id: usize
}

pub struct MediaView {
	media: Option<MediaViewResponse>,
}

impl Component for MediaView {
	type Message = Msg;
	type Properties = Property;

	fn create(ctx: &Context<Self>) -> Self {
		Self {
			media: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::RetrieveMediaView(value) => {
				self.media = Some(value);
			}
		}

		true
	}

	fn view(&self, _ctx: &Context<Self>) -> Html {
		if let Some(MediaViewResponse { metadata, media, progress }) = self.media.as_ref() {
			let media_prog = media.iter().zip(progress.iter());

			html! {
				<div class="media-view-container">
					<div class="info-container">
						<div class="thumbnail">
							<img src={ metadata.get_thumb_url() } />
						</div>
						<div class="metadata">
							<h3 class="title">{ metadata.get_title() }</h3>
							<p class="description">{ metadata.description.clone().unwrap_or_default() }</p>
						</div>
					</div>

					<section>
						<h2>{ "Files" }</h2>
						<div class="files-container">
							{
								for media_prog.map(|(media, prog)| {
									html! {
										<Link<Route> to={Route::ReadBook { book_id: media.id as usize }} classes={ classes!("file-item") }>
											<h5>{ media.file_name.clone() }</h5>
											<div><b>{ "File Size: " }</b>{ file_size_bytes_to_readable_string(media.file_size) }</div>
											<div><b>{ "File Type: " }</b>{ media.file_type.clone() }</div>
										</Link<Route>>
									}
								})
							}
						</div>
					</section>

					<section>
						<h2>{ "Characters" }</h2>
						<div class="characters-container">
							<div class="person-item">
								<div class="photo"><img src="/images/missingperson.jpg" /></div>
								<span class="title">{ "Character #1" }</span>
							</div>
							<div class="person-item">
								<div class="photo"><img src="/images/missingperson.jpg" /></div>
								<span class="title">{ "Character #2" }</span>
							</div>
						</div>
					</section>

					<section>
						<h2>{ "Creators" }</h2>
						<div class="authors-container">
							{
								for metadata.cached.author.clone().into_iter().map(|name| {
									html! {
										<div class="person-item">
											<div class="photo"><img src="/images/missingperson.jpg" /></div>
											<span class="title">{ name }</span>
										</div>
									}
								})
							}
						</div>
					</section>
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
			let metadata_id = ctx.props().id;

			ctx.link().send_future(async move {
				Msg::RetrieveMediaView(request::get_media_view(metadata_id).await)
			});
		}
	}
}

impl MediaView {
}