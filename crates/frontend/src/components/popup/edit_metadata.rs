use books_common::api::MediaViewResponse;
use yew::prelude::*;

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
	// RetrievePostersResponse()

	// Events
	SwitchTab(TabDisplay),
}


pub struct PopupEditMetadata {
	tab_display: TabDisplay,

	// cached_posters: Option<>,
}

impl Component for PopupEditMetadata {
	type Message = Msg;
	type Properties = Property;

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			tab_display: TabDisplay::General,
			// cached_posters: None,
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::SwitchTab(value) => self.tab_display = value,
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
					<h1>{"Edit"}</h1>
				</div>

				<div class="tab-bar">
					<div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::General))}>{ "General" }</div>
					<div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::Poster))}>{ "Poster" }</div>
					<div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::Info))}>{ "Info" }</div>
				</div>

				<div class="content">
					{ self.render_tab_contents(ctx.props()) }
				</div>
				<div class="footer">
					<button class="button">{ "Cancel" }</button>
					<button class="button">{ "Save" }</button>
				</div>
			</Popup>
		}
	}
}

impl PopupEditMetadata {
	fn render_tab_contents(&self, props: &<Self as Component>::Properties) -> Html {
		match self.tab_display {
			TabDisplay::General => self.render_tab_general(props),
			TabDisplay::Poster => self.render_tab_poster(props),
			TabDisplay::Info => self.render_tab_info(props),
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

	fn render_tab_poster(&self, _props: &<Self as Component>::Properties) -> Html {
		html! {
			<div class="content edit-posters">
				<div class="drop-container">
					<h4>{ "Drop File To Upload" }</h4>
				</div>
				<div class="poster-list">
					//
				</div>
			</div>
		}
	}

	fn render_tab_info(&self, _props: &<Self as Component>::Properties) -> Html {
		html! {
			<div class="content">
			</div>
		}
	}
}