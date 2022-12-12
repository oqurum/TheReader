use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_router::prelude::*;

use common::{
    api::WrappingResponse,
    component::{Popup, PopupType},
};
use common_local::{api, Collection};

use crate::{request, BaseRoute};

#[derive(Clone)]
pub enum Msg {
    ListResults(WrappingResponse<api::ApiGetCollectionListResponse>),

    OpenPopup,
    ClosePopup,
}

pub struct CollectionListPage {
    items: Option<Vec<Collection>>,
    display_popup: bool,
}

impl Component for CollectionListPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            items: None,
            display_popup: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ListResults(resp) => {
                match resp.ok() {
                    Ok(resp) => {
                        self.items = Some(resp);
                    }

                    Err(err) => crate::display_error(err),
                }

                self.display_popup = false;
            }

            Msg::OpenPopup => self.display_popup = true,
            Msg::ClosePopup => self.display_popup = false,
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="view-container">
                    <div>
                        <button class="btn btn-primary" onclick={ ctx.link().callback(|_| Msg::OpenPopup) }>{ "New Collection" }</button>
                    </div>
                    { self.render_main(ctx) }
                </div>

                {
                    if self.display_popup {
                        html! {
                            <CreateCollectionPopup
                                on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                on_submit={ ctx.link().callback_future(|value|  async move {
                                if let Err(e) = request::create_collection(&value).await.ok() {
                                    crate::display_error(e);
                                }

                                Msg::ListResults(request::get_collections().await)
                            }) } />
                        }
                    } else {
                        html! {}
                    }
                }
            </>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link()
                .send_future(async move { Msg::ListResults(request::get_collections().await) });
        }
    }
}

impl CollectionListPage {
    fn render_main(&self, _ctx: &Context<Self>) -> Html {
        if let Some(items) = self.items.as_deref() {
            html! {
                <nav class="nav flex-column">
                    {
                        for items.iter().map(|item| html! {
                            <Link<BaseRoute> to={ BaseRoute::ViewCollection { id: item.id } } classes={ "nav-link link-light" }>
                                <span>{ item.name.clone() }</span>
                            </Link<BaseRoute>>
                        })
                    }
                </nav>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }
}

#[derive(Properties, PartialEq)]
struct CreatePopupProps {
    on_submit: Callback<api::NewCollectionBody>,
    on_close: Callback<()>,
}

#[function_component(CreateCollectionPopup)]
fn _create_popup(props: &CreatePopupProps) -> Html {
    let name_ref = use_node_ref();
    let description_ref = use_node_ref();

    let on_save = {
        let name_ref = name_ref.clone();
        let description_ref = description_ref.clone();

        props.on_submit.reform(move |_| {
            let name_input = name_ref.cast::<HtmlInputElement>().unwrap();
            let description_textarea = description_ref.cast::<HtmlTextAreaElement>().unwrap();

            api::NewCollectionBody {
                name: name_input.value(),
                description: Some(description_textarea.value().trim().to_string())
                    .filter(|v| !v.is_empty()),
            }
        })
    };

    html! {
        <Popup type_of={ PopupType::FullOverlay } on_close={ props.on_close.clone() }>
            <div class="modal-body">
                <div class="mb-3">
                    <label class="form-label">{ "Name" }</label>
                    <input class="form-control" ref={ name_ref } type="text" placeholder="Container Name" />
                </div>

                <div class="mb-3">
                    <label class="form-label">{ "Description" }</label>
                    <textarea class="form-control" ref={ description_ref } placeholder="Container Description"></textarea>
                </div>
            </div>

            <div class="modal-footer">
                <button class="btn btn-success" onclick={ on_save }>{ "Create" }</button>
            </div>
        </Popup>
    }
}
