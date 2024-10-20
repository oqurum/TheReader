use common::{
    api::WrappingResponse,
    component::{ExpandableContainerComponent, Popup, PopupType},
    BookId,
};
use common_local::{
    api::{self, GetBookResponse},
    util::file_size_bytes_to_readable_string,
    FileId, Progression, ThumbnailStoreExt,
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

#[derive(Clone, PartialEq)]
pub enum LocalPopup {
    Poster(DisplayOverlayItem),
    UseExistingProgress { file_id: FileId },
}

#[derive(Clone)]
pub enum Msg {
    // Retrive
    RetrieveMediaView(Box<WrappingResponse<GetBookResponse>>),

    // Events
    ShowPopup(LocalPopup),
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

    media_popup: Option<LocalPopup>,
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
                if let Some(old_disp) = self.media_popup.as_ref() {
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
            <>
                { self.render_popup(ctx) }
                <div class="view-container">
                    { self.render_main(ctx) }
                </div>
            </>
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

    fn changed(&mut self, ctx: &Context<Self>, _prev: &Self::Properties) -> bool {
        let book_id = ctx.props().id;

        ctx.link().send_future(async move {
            Msg::RetrieveMediaView(Box::new(request::get_media_view(book_id).await))
        });

        true
    }
}

impl BookPage {
    fn render_popup(&self, ctx: &Context<Self>) -> Html {
        let Some(GetBookResponse { book, progress, .. }) = self.media.as_ref() else {
            return html! {};
        };

        let Some(overlay_type) = self.media_popup.as_ref() else {
            return html! {};
        };

        match overlay_type {
            &LocalPopup::UseExistingProgress { file_id } => {
                html! {
                    <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                        <div class="d-flex flex-column p-2">
                            <h4>{ "Book Progression" }</h4>

                            <p>{ "Progress existing on another File. If would you like to transfer progress from another file, select below or start reading." }</p>
                            <small>{ "Note: Progress from one file may not align to the new file." }</small>

                            <div class="d-flex flex-column">
                                {
                                    for progress.iter().map(|progress| html! {
                                        <div class="p-2">
                                            {
                                                if let Some(&Progression::Ebook { chapter, page, .. }) = progress.as_ref() {
                                                    html! {
                                                        <>
                                                            <span>{ "Chapter: " } { chapter }</span>
                                                            <br />
                                                            <span>{ "Page: " } { page }</span>
                                                        </>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                        </div>

                                    })
                                }
                            </div>

                            <Link<BaseRoute> to={ BaseRoute::ReadBook { book_id: file_id } } classes="btn btn-success">
                                <span>{ "Start From Beginning" }</span>
                            </Link<BaseRoute>>
                        </div>
                    </Popup>
                }
            }

            LocalPopup::Poster(DisplayOverlayItem::Info { book_id: _ }) => {
                html! {
                    <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                        <h1>{"Info"}</h1>
                    </Popup>
                }
            }

            &LocalPopup::Poster(DisplayOverlayItem::More {
                book_id,
                mouse_pos: (pos_x, pos_y),
            }) => {
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
                                DropdownInfoPopupEvent::MarkAsRead => Msg::ClosePopup,
                                DropdownInfoPopupEvent::MarkAsUnread => Msg::ClosePopup,

                                DropdownInfoPopupEvent::Closed => Msg::ClosePopup,
                                DropdownInfoPopupEvent::RefreshMetadata => Msg::UpdateBook(book_id),
                                DropdownInfoPopupEvent::UnMatchBook => Msg::UnMatchBook(book_id),
                                DropdownInfoPopupEvent::SearchFor => Msg::ShowPopup(LocalPopup::Poster(DisplayOverlayItem::SearchForBook { book_id, input_value: None })),
                                DropdownInfoPopupEvent::Info => Msg::ShowPopup(LocalPopup::Poster(DisplayOverlayItem::Info { book_id })),
                            }
                        }) }
                    />
                }
            }

            LocalPopup::Poster(DisplayOverlayItem::Edit(resp)) => {
                html! {
                    <PopupEditBook
                        on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                        classes={ classes!("popup-book-edit") }
                        media_resp={ (**resp).clone() }
                    />
                }
            }

            &LocalPopup::Poster(DisplayOverlayItem::SearchForBook {
                book_id,
                ref input_value,
            }) => {
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
    }

    fn render_main(&self, ctx: &Context<Self>) -> Html {
        if let Some(GetBookResponse {
            people,
            book,
            media,
            progress: progress_vec,
        }) = self.media.as_ref()
        {
            let book_id = book.id;
            let on_click_more = ctx.link().callback(move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();

                Msg::ShowPopup(LocalPopup::Poster(DisplayOverlayItem::More {
                    book_id,
                    mouse_pos: (e.page_x(), e.page_y()),
                }))
            });

            html! {
                <div class="item-view-container">
                    <div class="g-2 p-2 row">
                        <div class="poster large p-0">
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
                                            Ok(resp) => Msg::ShowPopup(LocalPopup::Poster(DisplayOverlayItem::Edit(Box::new(resp)))),
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
                        <div class="col-sm-12 col-md metadata-container">
                            <h1 class="title">{ book.get_title() }</h1>

                            <div class="badge-list">
                                {
                                    if let Some(url) = book.public_source_url.clone() {
                                        html! {
                                            <a href={url} target="_blank" class="badge bg-secondary link-light">{ "Source" }</a>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }
                            </div>

                            <ExpandableContainerComponent max_expanded_lines=11 overflow_scroll=true>
                                { book.description.clone().unwrap_or_default() }
                            </ExpandableContainerComponent>
                        </div>
                    </div>

                    <section>
                        <h2>{ "Files" }</h2>
                        <div class="row">
                            {
                                for media.iter().zip(progress_vec.iter()).map(|(media, progress)| {
                                    let file_id = media.id;
                                    let nav = ctx.link().navigator().unwrap();
                                    let route = BaseRoute::ReadBook { book_id: media.id };
                                    let has_progress = progress.is_some();
                                    let other_has_progress = progress_vec.iter().any(|v| v.is_some());

                                    html! {
                                        <a
                                            href={ route.to_path() }
                                            class={ "col-sm-12 col-md-6 col-lg-4 file-item link-light" }
                                            onclick={ ctx.link().callback(move |e: MouseEvent| {
                                                e.prevent_default();

                                                if !has_progress && other_has_progress {
                                                    Msg::ShowPopup(LocalPopup::UseExistingProgress {
                                                        file_id,
                                                    })
                                                } else {
                                                    nav.push(&route);
                                                    Msg::Ignore
                                                }
                                            }) }
                                        >
                                            <h5>{ media.file_name.clone() }</h5>
                                            <div><b>{ "File Size: " }</b>{ file_size_bytes_to_readable_string(media.file_size) }</div>
                                            <div><b>{ "File Type: " }</b>{ media.file_type.clone() }</div>
                                            {
                                                if let Some(&Progression::Ebook { chapter, .. }) = progress.as_ref() {
                                                    html! {
                                                        <div
                                                            style="margin-top: 3px;background-color: var(--bs-body-color);height: 5px;overflow: hidden;border-radius: 8px;"
                                                            title={ format!("{chapter}/{}", media.chapter_count) }
                                                        >
                                                            <div style={format!("background-color: var(--bs-green); height: 100%; width: {}%;", chapter as f32 / media.chapter_count as f32 * 100.0)}></div>
                                                        </div>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                        </a>
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
                                            <div class="photo"><img src={ person.get_thumb_url() } /></div>
                                            <span class="title">{ person.name.clone() }</span>
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
                <div class="item-view-container">
                    <div class="g-2 p-2 row">
                        <div class="poster large p-0">
                            <img class="rounded placeholder" />
                        </div>
                        <div class="col-sm-12 col-md metadata-container placeholder-glow">
                            <h4 class="title placeholder col-2"></h4>

                            <div class="badge-list mb-2">
                                <a class="badge placeholder">{ "EMPTY" }</a>
                            </div>

                            <div class="d-flex flex-column">
                                <div class="placeholder col-3 mb-1"></div>
                                <div class="placeholder col-3 mb-1"></div>
                                <div class="placeholder col-2 mb-1"></div>
                                <div class="placeholder col-3"></div>
                            </div>
                        </div>
                    </div>

                    <section>
                        <h2>{ "Files" }</h2>
                        <div class="row">
                            {
                                for (0..2).map(|_| {
                                    html! {
                                        <a class={ "d-flex flex-column col-sm-12 col-md-6 col-lg-4 link-light placeholder-glow" }>
                                            <h5 class="placeholder col-6"></h5>
                                            <h6 class="placeholder col-4"></h6>
                                            <div class="placeholder col-3"></div>
                                        </a>
                                    }
                                })
                            }
                        </div>
                    </section>

                    <section>
                        <h2>{ "People" }</h2>
                        <div class="authors-container">
                            {
                                for (0..2).into_iter().map(|_| {
                                    html! {
                                        <div class="person-container placeholder-glow">
                                            <div class="photo"><img class="placeholder" /></div>
                                            <span class="title placeholder"></span>
                                        </div>
                                    }
                                })
                            }
                        </div>
                    </section>
                </div>
            }
        }
    }
}
