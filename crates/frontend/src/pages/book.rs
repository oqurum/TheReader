use common::{
    api::WrappingResponse,
    component::{ExpandableContainerComponent, Popup, PopupType},
    BookId,
};
use common_local::{
    api::{self, GetBookResponse},
    util::file_size_bytes_to_readable_string,
    ThumbnailStoreExt, Progression,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    components::{
        book_poster_item::DisplayOverlayItem, DropdownInfoPopup, DropdownInfoPopupEvent,
        PopupEditBook, PopupSearchBook,
    },
    request, BaseRoute,
};

#[derive(Clone)]
pub enum Msg {
    // Retrive
    RetrieveMediaView(Box<WrappingResponse<GetBookResponse>>),

    // Events
    ShowPopup(DisplayOverlayItem),
    ClosePopup,

    // TODO: Replace with book_poster_item::PosterItem
    // Popup Events
    UpdateBook(BookId),
    UnMatchBook(BookId),

    Ignore,
}

#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub id: BookId,
}

pub struct BookPage {
    media: Option<GetBookResponse>,

    media_popup: Option<DisplayOverlayItem>,
}

impl Component for BookPage {
    type Message = Msg;
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            media: None,
            media_popup: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => return false,

            Msg::ClosePopup => {
                self.media_popup = None;
            }

            Msg::RetrieveMediaView(value) => match value.ok() {
                Ok(resp) => self.media = Some(resp),
                Err(err) => crate::display_error(err),
            },

            Msg::ShowPopup(new_disp) => {
                if let Some(old_disp) = self.media_popup.as_mut() {
                    if *old_disp == new_disp {
                        self.media_popup = None;
                    } else {
                        self.media_popup = Some(new_disp);
                    }
                } else {
                    self.media_popup = Some(new_disp);
                }
            }

            Msg::UpdateBook(book_id) => {
                ctx.link().send_future(async move {
                    request::update_book(book_id, &api::PostBookBody::AutoMatchBookId).await;

                    Msg::Ignore
                });
            }

            Msg::UnMatchBook(book_id) => {
                ctx.link().send_future(async move {
                    request::update_book(book_id, &api::PostBookBody::UnMatch).await;

                    Msg::Ignore
                });
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="view-container">
                { self.render_main(ctx) }
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let book_id = ctx.props().id;

            ctx.link().send_future(async move {
                Msg::RetrieveMediaView(Box::new(request::get_media_view(book_id).await))
            });
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let book_id = ctx.props().id;

        ctx.link().send_future(async move {
            Msg::RetrieveMediaView(Box::new(request::get_media_view(book_id).await))
        });

        true
    }
}

impl BookPage {
    fn render_main(&self, ctx: &Context<Self>) -> Html {
        if let Some(GetBookResponse {
            people,
            book,
            media,
            progress,
        }) = self.media.as_ref()
        {
            let book_id = book.id;
            let on_click_more = ctx.link().callback(move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();

                Msg::ShowPopup(DisplayOverlayItem::More {
                    book_id,
                    mouse_pos: (e.page_x(), e.page_y()),
                })
            });

            html! {
                <div class="item-view-container">
                    <div class="info-container">
                        <div class="poster large">
                            <div class="bottom-right">
                                <span class="material-icons" onclick={on_click_more} title="More Options">{ "more_horiz" }</span>
                            </div>
                            <div class="bottom-left">
                                <span class="material-icons" onclick={ctx.link().callback_future(move |e: MouseEvent| {
                                    e.prevent_default();
                                    e.stop_propagation();

                                    async move {
                                        let resp = request::get_media_view(book_id).await;

                                        match resp.ok() {
                                            Ok(resp) => Msg::ShowPopup(DisplayOverlayItem::Edit(Box::new(resp))),
                                            Err(err) => {
                                                crate::display_error(err);

                                                Msg::Ignore
                                            }
                                        }
                                    }
                                })} title="More Options">{ "edit" }</span>
                            </div>

                            <img class="rounded" src={ book.thumb_path.get_book_http_path().into_owned() } />
                        </div>
                        <div class="metadata-container">
                            <div class="metadata">
                                <h1 class="title">{ book.get_title() }</h1>
                                <ExpandableContainerComponent>
                                    { book.description.clone().unwrap_or_default() }
                                </ExpandableContainerComponent>
                            </div>
                        </div>
                    </div>

                    <section>
                        <h2>{ "Files" }</h2>
                        <div class="files-container">
                            {
                                for media.iter().zip(progress.iter()).map(|(media, progress)| {
                                    // TODO: Add an "Are you sure?" for if we clicked on a non-progressed file.

                                    html! {
                                        <Link<BaseRoute> to={ BaseRoute::ReadBook { book_id: media.id } } classes={ "file-item link-light" }>
                                            <h5>{ media.file_name.clone() }</h5>
                                            <div><b>{ "File Size: " }</b>{ file_size_bytes_to_readable_string(media.file_size) }</div>
                                            <div><b>{ "File Type: " }</b>{ media.file_type.clone() }</div>
                                            {
                                                if let Some(&Progression::Ebook { chapter, .. }) = progress.as_ref() {
                                                    html! {
                                                        <div
                                                            style="background-color: var(--bs-body-color);height: 5px;overflow: hidden;border-radius: 8px;"
                                                            title={ format!("{chapter}/{}", media.chapter_count) }
                                                        >
                                                            <div style={format!("background-color: var(--bs-green); height: 100%; width: {}%;", chapter as f32 / media.chapter_count as f32 * 100.0)}></div>
                                                        </div>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                        </Link<BaseRoute>>
                                    }
                                })
                            }
                        </div>
                    </section>

                    <section>
                        <h2>{ "People" }</h2>
                        <div class="authors-container">
                            {
                                for people.iter().map(|person| {
                                    html! {
                                        <div class="person-container">
                                            <div class="photo"><img class="rounded" src={ person.get_thumb_url() } /></div>
                                            <span class="title">{ person.name.clone() }</span>
                                        </div>
                                    }
                                })
                            }
                        </div>
                    </section>

                    {
                        if let Some(overlay_type) = self.media_popup.as_ref() {
                            match overlay_type {
                                DisplayOverlayItem::Info { book_id: _ } => {
                                    html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <h1>{"Info"}</h1>
                                        </Popup>
                                    }
                                }

                                &DisplayOverlayItem::More { book_id, mouse_pos: (pos_x, pos_y) } => {
                                    let is_matched = book.source.agent.as_ref() != "local";

                                    html! {
                                        <DropdownInfoPopup
                                            { pos_x }
                                            { pos_y }

                                            { book_id }
                                            { is_matched }

                                            event={ ctx.link().callback(move |e| {
                                                match e {
                                                    // TODO:
                                                    DropdownInfoPopupEvent::AddToCollection(_id) => Msg::ClosePopup,
                                                    DropdownInfoPopupEvent::RemoveFromCollection(_id) => Msg::ClosePopup,
                                                    DropdownInfoPopupEvent::Closed => Msg::ClosePopup,
                                                    DropdownInfoPopupEvent::RefreshMetadata => Msg::UpdateBook(book_id),
                                                    DropdownInfoPopupEvent::UnMatchBook => Msg::UnMatchBook(book_id),
                                                    DropdownInfoPopupEvent::SearchFor => Msg::ShowPopup(DisplayOverlayItem::SearchForBook { book_id, input_value: None }),
                                                    DropdownInfoPopupEvent::Info => Msg::ShowPopup(DisplayOverlayItem::Info { book_id }),
                                                }
                                            }) }
                                        />
                                    }
                                }

                                DisplayOverlayItem::Edit(resp) => {
                                    html! {
                                        <PopupEditBook
                                            on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                            classes={ classes!("popup-book-edit") }
                                            media_resp={ (**resp).clone() }
                                        />
                                    }
                                }

                                &DisplayOverlayItem::SearchForBook { book_id, ref input_value } => {
                                    let input_value = if let Some(v) = input_value {
                                        v.to_string()
                                    } else {
                                        format!(
                                            "{} {}",
                                            book.title.as_deref().unwrap_or_default(),
                                            book.cached.author.as_deref().unwrap_or_default()
                                        )
                                    };

                                    let input_value = input_value.trim().to_string();

                                    html! {
                                        <PopupSearchBook {book_id} {input_value} on_close={ ctx.link().callback(|_| Msg::ClosePopup) } />
                                    }
                                }
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }
}
