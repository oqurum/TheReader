use books_common::{api::{MediaViewResponse, GetPostersResponse}, Either};
use yew::prelude::*;

use crate::request;

use super::{Popup, PopupType};


#[derive(Clone, Copy)]
pub enum TabDisplay {
	General,
	Poster,
	Info,
}


#[derive(Properties, PartialEq)]
pub struct Property {
	#[prop_or_default]
    pub classes: Classes,

	pub on_close: Callback<()>,

	pub media_resp: MediaViewResponse,
}


pub enum Msg {
	RetrievePostersResponse(GetPostersResponse),

	// Events
	SwitchTab(TabDisplay),

	UpdatedPoster,

	Ignore,
}


pub struct PopupEditMetadata {
	tab_display: TabDisplay,

	cached_posters: Option<GetPostersResponse>,
}

impl Component for PopupEditMetadata {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			tab_display: TabDisplay::General,
			cached_posters: None,
		}
	}

	fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::Ignore => {
				return false;
			}

			Msg::SwitchTab(value) => {
				self.tab_display = value;
				self.cached_posters = None;
			}

			Msg::RetrievePostersResponse(resp) => {
				self.cached_posters = Some(resp);
			}

			Msg::UpdatedPoster => {
				let meta_id = ctx.props().media_resp.metadata.id;

				ctx.link()
				.send_future(async move {
					Msg::RetrievePostersResponse(request::get_posters_for_meta(meta_id).await)
				});

				return false;
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		html! {
			<Popup
				type_of={ PopupType::FullOverlay }
				on_close={ ctx.props().on_close.clone() }
				classes={ classes!("popup-book-edit") }
			>
				<div class="header">
					<h2>{"Edit"}</h2>
				</div>

				<div class="tab-bar">
					<div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::General))}>{ "General" }</div>
					<div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::Poster))}>{ "Poster" }</div>
					<div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::Info))}>{ "Info" }</div>
				</div>

				{ self.render_tab_contents(ctx) }

				<div class="footer">
					<button class="button">{ "Cancel" }</button>
					<button class="button">{ "Save" }</button>
				</div>
			</Popup>
		}
	}
}

impl PopupEditMetadata {
	fn render_tab_contents(&self, ctx: &Context<Self>) -> Html {
		match self.tab_display {
			TabDisplay::General => self.render_tab_general(ctx.props()),
			TabDisplay::Poster => {
				if self.cached_posters.is_none() {
					let metadata_id = ctx.props().media_resp.metadata.id;

					ctx.link()
					.send_future(async move {
						Msg::RetrievePostersResponse(request::get_posters_for_meta(metadata_id).await)
					});
				}

				self.render_tab_poster(ctx)
			},
			TabDisplay::Info => self.render_tab_info(ctx.props()),
		}
	}


	fn render_tab_general(&self, props: &<Self as Component>::Properties) -> Html {
		let resp = &props.media_resp;

		html! {
			<div class="content">
				<label for="input-title">{ "Title" }</label>
				<input type="text" id="input-title" value={ resp.metadata.title.clone().unwrap_or_default() } />

				<label for="input-orig-title">{ "Original Title" }</label>
				<input type="text" id="input-orig-title" value={ resp.metadata.original_title.clone().unwrap_or_default() } />

				<label for="input-descr">{ "Description" }</label>
				<textarea type="text" id="input-descr" rows="5" value={ resp.metadata.description.clone().unwrap_or_default() } />
			</div>
		}
	}

	fn render_tab_poster(&self, ctx: &Context<Self>) -> Html {
		if let Some(resp) = self.cached_posters.as_ref() {
			html! {
				<div class="content edit-posters">
					<div class="drop-container">
						<h4>{ "Drop File To Upload" }</h4>
					</div>
					<div class="poster-list">
						{
							for resp.items.iter().map(|poster| {
								let meta_id = ctx.props().media_resp.metadata.id;
								let url_or_id = poster.id.map(Either::Right).unwrap_or_else(|| Either::Left(poster.path.clone()));
								let is_selected = poster.selected;

								html_nested! {
									<div
										class={ classes!("poster", { if is_selected { "selected" } else { "" } }) }
										onclick={ctx.link().callback_future(move |_| {
											let url_or_id = url_or_id.clone();

											async move {
												if is_selected {
													Msg::Ignore
												} else {
													request::change_poster_for_meta(meta_id, url_or_id).await;

													Msg::UpdatedPoster
												}
											}
										})}
									>
										<img src={poster.path.clone()} />
									</div>
								}
							})
						}
					</div>
				</div>
			}
		} else {
			html! {
				<div class="content edit-posters">
					<h3>{ "Loading Posters..." }</h3>
				</div>
			}
		}
	}

	fn render_tab_info(&self, _props: &<Self as Component>::Properties) -> Html {
		html! {
			<div class="content">
			</div>
		}
	}
}