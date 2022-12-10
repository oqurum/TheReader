use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use common::{
    api::WrappingResponse,
    component::{Popup, PopupType},
};
use common_local::{api, Collection, CollectionId, DisplayItem};

use crate::{
    components::{BookPosterItem, Sidebar},
    request,
};

#[derive(Properties, PartialEq, Eq)]
pub struct Props {
    pub id: CollectionId,
}

#[derive(Clone)]
pub enum Msg {
    ItemResults(WrappingResponse<api::ApiGetCollectionIdResponse>),
    BooksResults(WrappingResponse<api::GetBookListResponse>),

    OpenPopup,
    ClosePopup,
}

pub struct CollectionItemPage {
    media_items: Option<Vec<DisplayItem>>,
    item: Option<Collection>,
    display_popup: bool,
}

impl Component for CollectionItemPage {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            media_items: None,
            item: None,
            display_popup: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ItemResults(resp) => {
                match resp.ok() {
                    Ok(resp) => {
                        let id = ctx.props().id;
                        ctx.link().send_future(async move {
                            Msg::BooksResults(request::get_collection_books(id).await)
                        });

                        self.item = Some(resp);
                    }

                    Err(err) => crate::display_error(err),
                }

                self.display_popup = false;
            }

            Msg::BooksResults(resp) => match resp.ok() {
                Ok(resp) => {
                    self.media_items = Some(resp.items);
                }

                Err(err) => crate::display_error(err),
            },

            Msg::OpenPopup => self.display_popup = true,
            Msg::ClosePopup => self.display_popup = false,
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <Sidebar />
                <div class="view-container">
                    {
                        if let Some(info) = self.item.as_ref() {
                            self.render_main(info, ctx)
                        } else {
                            html! {
                                <h2>{ "Loading.." }</h2>
                            }
                        }
                    }
                </div>

                {
                    if self.display_popup {
                        let id = ctx.props().id;

                        html! {
                            <CreateCollectionPopup
                                on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                on_submit={ ctx.link().callback_future(move |value| async move {
                                    if let Err(e) = request::create_collection(&value).await.ok() {
                                        crate::display_error(e);
                                    }

                                    Msg::ItemResults(request::get_collection(id).await)
                                })
                            } />
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let id = ctx.props().id;

            ctx.link()
                .send_future(async move { Msg::ItemResults(request::get_collection(id).await) });
        }
    }
}

impl CollectionItemPage {
    fn render_main(&self, info: &Collection, _ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="collection-list">
                    <h2>{ info.name.clone() }</h2>
                    <p>{ info.description.clone().unwrap_or_default() }</p>
                </div>

                <div class="book-list normal">
                    {
                        if let Some(items) = self.media_items.as_ref() {
                            html! {
                                for items.iter().map(|item| {
                                    html! {
                                        <BookPosterItem item={ item.clone() } />
                                    }
                                })
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
            </>
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
            <div class="form-container">
                <label>{ "Name" }</label>
                <input ref={ name_ref } type="text" placeholder="Container Name" />
            </div>

            <div class="form-container">
                <label>{ "Description" }</label>
                <textarea ref={ description_ref } placeholder="Container Description"></textarea>
            </div>

            <button class="green" onclick={ on_save }>{ "Create" }</button>
        </Popup>
    }
}
