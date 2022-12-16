use common::{
    api::WrappingResponse,
    component::{Popup, PopupType},
    util::{truncate_on_indices, LoadingItem},
    BookId,
};
use common_local::{
    api::{BookSearchResponse, PostBookBody, SearchItem},
    SearchType,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{components::BookListItemInfo, request};

#[derive(Properties, PartialEq)]
pub struct Property {
    #[prop_or_default]
    pub classes: Classes,

    pub on_close: Callback<()>,

    pub book_id: BookId,
    pub input_value: String,
}

pub enum Msg {
    BookSearchResponse(String, WrappingResponse<BookSearchResponse>),

    SearchFor(String),

    Ignore,
}

pub struct PopupSearchBook {
    cached_posters: Option<LoadingItem<BookSearchResponse>>,
    input_value: String,
}

impl Component for PopupSearchBook {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link()
            .send_message(Msg::SearchFor(ctx.props().input_value.clone()));

        Self {
            cached_posters: None,
            input_value: ctx.props().input_value.clone(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => {
                return false;
            }

            Msg::SearchFor(search) => {
                self.cached_posters = Some(LoadingItem::Loading);

                ctx.link().send_future(async move {
                    let resp = request::search_for(&search, SearchType::Book).await;

                    Msg::BookSearchResponse(search, resp)
                });
            }

            Msg::BookSearchResponse(search, resp) => match resp.ok() {
                Ok(resp) => {
                    self.cached_posters = Some(LoadingItem::Loaded(resp));
                    self.input_value = search;
                }

                Err(err) => crate::display_error(err),
            },
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let input_ref = NodeRef::default();

        html! {
            <Popup
                type_of={ PopupType::FullOverlay }
                on_close={ ctx.props().on_close.clone() }
                classes={ classes!("external-book-search-popup") }
            >
                <div class="modal-header">
                    <h5 class="modal-title">{ "Book Search" }</h5>
                </div>

                <div class="modal-body">
                    <form>
                        <div class="input-group mb-3">
                            <input ref={ input_ref.clone() }
                                type="text" class="form-control"
                                placeholder="Search For Book Title"
                                value={ self.input_value.clone() }
                            />
                            <button class="btn btn-primary" onclick={
                                ctx.link().callback(move |e: MouseEvent| {
                                    e.prevent_default();

                                    let input = input_ref.cast::<HtmlInputElement>().unwrap();

                                    Msg::SearchFor(input.value())
                                })
                            }>{ "Search" }</button>
                        </div>
                    </form>

                    <div class="external-book-search-container">
                        {
                            if let Some(resp) = self.cached_posters.as_ref() {
                                match resp {
                                    LoadingItem::Loaded(resp) => html! {
                                        <>
                                            <h2>{ "Results" }</h2>
                                            <div class="book-search-items">
                                            {
                                                for resp.items.iter()
                                                    .flat_map(|(name, values)| values.iter().map(|v| (name.clone(), v)))
                                                    .map(|(site, item)| Self::render_poster_container(site, item, ctx))
                                            }
                                            </div>
                                        </>
                                    },

                                    LoadingItem::Loading => html! {
                                        <h2>{ "Loading..." }</h2>
                                    }
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                </div>
            </Popup>
        }
    }
}

impl PopupSearchBook {
    fn render_poster_container(site: String, item: &SearchItem, ctx: &Context<Self>) -> Html {
        let item = item.as_book();
        let source = item.source.clone();

        let book_id = ctx.props().book_id;

        html! {
            <BookListItemInfo
                onclick_close_popup=true
                image={ Some(item.thumbnail_url.to_string()).filter(|v| !v.trim().is_empty()) }
                title={ item.name.clone() }
                subtitle={ site }
                description={ item.description.clone()
                    .map(|mut v| { truncate_on_indices(&mut v, 300); v })
                    .unwrap_or_default() }
                onclick={
                    ctx.link()
                    .callback_future(move |_| {
                        let source = source.clone();

                        async move {
                            request::update_book(
                                book_id,
                                &PostBookBody::UpdateBookBySource(source)
                            ).await;

                            Msg::Ignore
                        }
                    })
                }
            />
        }
    }
}
