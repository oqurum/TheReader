use yew::prelude::*;

pub enum Msg {
}

pub struct LoginPage {
}

impl Component for LoginPage {
	type Message = Msg;
	type Properties = ();

	fn create(_ctx: &Context<Self>) -> Self {
		Self {}
	}

	fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
		true
	}

	fn view(&self, _ctx: &Context<Self>) -> Html {
		html! {
			<div class="login-container">
				<div class="center-normal">
					<div class="center-container">
						<h2>{ "Passwordless Login" }</h2>
						<form action="/auth/passwordless" method="post">
							<label for="email">{ "Email Address" }</label>
							<input type="email" name="email" id="email" />
							<input type="submit" value="Log in" class="button" />
						</form>

						<h2>{ "Password Login" }</h2>
						<form action="/auth/password" method="post">
							<label for="email">{ "Email Address" }</label>
							<input type="email" name="email" id="email" />
							<label for="password">{ "Password" }</label>
							<input type="password" name="password" id="password" />
							<input type="submit" value="Log in" class="button" />
						</form>
					</div>
				</div>
			</div>
		}
	}
}