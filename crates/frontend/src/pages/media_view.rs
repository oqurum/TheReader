use common_local::{api::{GetBookResponse, self}, util::file_size_bytes_to_readable_string, ThumbnailStoreExt};
use common::{BookId, component::popup::{Popup, PopupClose, PopupType}, api::WrappingResponse};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{request, Route, components::{PopupSearchBook, PopupEditBook}, util::{on_click_prevdef, on_click_prevdef_stopprop}};

#[derive(Clone)]
pub enum Msg {
    // Retrive
    RetrieveMediaView(Box<WrappingResponse<GetBookResponse>>),

    // Events
    ShowPopup(DisplayOverlay),
    ClosePopup,

    // Popup Events
    UpdateBook(BookId),

    Ignore
}

#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub id: BookId
}

pub struct MediaView {
    media: Option<GetBookResponse>,

    media_popup: Option<DisplayOverlay>,
}

impl Component for MediaView {
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

            Msg::RetrieveMediaView(value) => {
                match value.ok() {
                    Ok(resp) => self.media = Some(resp),
                    Err(err) => crate::display_error(err),
                }
            }

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
                ctx.link()
                .send_future(async move {
                    request::update_book(book_id, &api::PostBookBody::AutoMatchBookId).await;

                    Msg::Ignore
                });
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <div class="sidebar-container"></div>
                <div class="view-container">
                    { self.render_main(ctx) }
                </div>
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

impl MediaView {
    fn render_main(&self, ctx: &Context<Self>) -> Html {
        if let Some(GetBookResponse { people, book, media, progress }) = self.media.as_ref() {
            let book_id = book.id;
            let on_click_more = ctx.link().callback(move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();

                Msg::ShowPopup(DisplayOverlay::More { book_id, mouse_pos: (e.page_x(), e.page_y()) })
            });

            let media_prog = media.iter().zip(progress.iter());

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
                                            Ok(resp) => Msg::ShowPopup(DisplayOverlay::Edit(Box::new(resp))),
                                            Err(err) => {
                                                crate::display_error(err);

                                                Msg::Ignore
                                            }
                                        }
                                    }
                                })} title="More Options">{ "edit" }</span>
                            </div>

                            <img src={ book.thumb_path.get_book_http_path().into_owned() } />
                        </div>
                        <div class="metadata-container">
                            <div class="metadata">
                                <h3 class="title">{ book.get_title() }</h3>
                                <p class="description">{ book.description.clone().unwrap_or_default() }</p>
                            </div>
                        </div>
                    </div>

                    <section>
                        <h2>{ "Files" }</h2>
                        <div class="files-container">
                            {
                                for media_prog.map(|(media, _prog)| {
                                    html! {
                                        <Link<Route> to={Route::ReadBook { book_id: media.id }} classes={ classes!("file-item") }>
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

                    {
                        if let Some(overlay_type) = self.media_popup.as_ref() {
                            match overlay_type {
                                DisplayOverlay::Info { book_id: _ } => {
                                    html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <h1>{"Info"}</h1>
                                        </Popup>
                                    }
                                }

                                &DisplayOverlay::More { book_id, mouse_pos } => {
                                    html! {
                                        <Popup type_of={ PopupType::AtPoint(mouse_pos.0, mouse_pos.1) } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <div class="menu-list">
                                                <PopupClose class="menu-item">{ "Start Reading" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef(ctx.link(), Msg::UpdateBook(book_id))
                                                }>{ "Refresh Metadata" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop(ctx.link(), Msg::ShowPopup(DisplayOverlay::SearchForBook { book_id, input_value: None }))
                                                }>{ "Search For Book" }</PopupClose>
                                                <PopupClose class="menu-item">{ "Delete" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop(ctx.link(), Msg::ShowPopup(DisplayOverlay::Info { book_id }))
                                                }>{ "Show Info" }</PopupClose>
                                            </div>
                                        </Popup>
                                    }
                                }

                                DisplayOverlay::Edit(resp) => {
                                    html! {
                                        <PopupEditBook
                                            on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                            classes={ classes!("popup-book-edit") }
                                            media_resp={ (&**resp).clone() }
                                        />
                                    }
                                }

                                &DisplayOverlay::SearchForBook { book_id, ref input_value } => {
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


#[derive(Clone)]
pub enum DisplayOverlay {
    Info {
        book_id: BookId
    },

    Edit(Box<api::GetBookResponse>),

    More {
        book_id: BookId,
        mouse_pos: (i32, i32)
    },

    SearchForBook {
        book_id: BookId,
        input_value: Option<String>,
    },
}

impl PartialEq for DisplayOverlay {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Info { book_id: l_id }, Self::Info { book_id: r_id }) => l_id == r_id,
            (Self::More { book_id: l_id, .. }, Self::More { book_id: r_id, .. }) => l_id == r_id,
            (
                Self::SearchForBook { book_id: l_id, input_value: l_val, .. },
                Self::SearchForBook { book_id: r_id, input_value: r_val, .. }
            ) => l_id == r_id && l_val == r_val,

            _ => false
        }
    }
}