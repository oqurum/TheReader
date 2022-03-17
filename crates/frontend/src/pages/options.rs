use books_common::api;
use yew::prelude::*;

use crate::request;

pub enum Msg {
	OptionsResults(api::GetOptionsResponse)
}

pub struct OptionsPage {
	resp: Option<api::GetOptionsResponse>
}

impl Component for OptionsPage {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {
			resp: None
		}
	}

	fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
		match msg {
			Msg::OptionsResults(resp) => {
				self.resp = Some(resp);
			}
		}

		true
	}

	fn view(&self, ctx: &Context<Self>) -> Html {
		if let Some(resp) = self.resp.as_ref() {
			html! {
				<div class="options-page">
					<h2>{ "Libraries" }</h2>
					{
						for resp.libraries.iter()
							.map(|v| html! {
								<>
									<h3>{ v.name.clone() }</h3>
									<ul>
										{ for v.directories.iter().map(|v| html! { <li>{ v.clone() }</li> }) }
										<li><button>{ "Add New" }</button></li>
									</ul>
								</>
							})
					}
					<button>{ "Add Library" }</button>
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
			.send_future(async {
				Msg::OptionsResults(request::get_options().await)
			});
		}
	}
}

impl OptionsPage {
	//
}
