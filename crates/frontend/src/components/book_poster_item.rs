use common::{
    component::{Popup, PopupClose, PopupType},
    BookId, Either,
};
use common_local::{api, DisplayItem, MediaItem, Progression, ThumbnailStoreExt};
use web_sys::{HtmlElement, HtmlInputElement, MouseEvent};
use yew::{function_component, html, Callback, Component, Context, Html, Properties, TargetCast};
use yew_router::prelude::Link;

use crate::{
    request,
    util::{on_click_prevdef_cb, on_click_prevdef_stopprop_cb},
    Route,
};

#[derive(Properties)]
pub struct BookPosterItemProps {
    // TODO: Convert to Either<DisplayItem, BookProgression> and remove progress field.
    pub item: DisplayItem,
    pub callback: Option<Callback<BookPosterItemMsg>>,

    // i64 is currently just total chapter count
    pub progress: Option<(Progression, MediaItem)>,

    #[prop_or_default]
    pub is_editing: bool,
    #[prop_or_default]
    pub is_updating: bool,
}

impl PartialEq for BookPosterItemProps {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
            && self.is_editing == other.is_editing
            && self.is_updating == other.is_updating
    }
}

#[derive(Clone)]
pub enum BookPosterItemMsg {
    PosterItem(PosterItem),

    AddOrRemoveItemFromEditing(BookId, bool),

    Ignore,
}

pub struct BookPosterItem;

impl Component for BookPosterItem {
    type Message = BookPosterItemMsg;
    type Properties = BookPosterItemProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        if let Some(cb) = ctx.props().callback.as_ref() {
            cb.emit(msg);
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let &BookPosterItemProps {
            is_updating,
            ref item,
            ..
        } = ctx.props();

        let route_to = if let Some((_, file)) = ctx.props().progress.as_ref() {
            Route::ReadBook { book_id: file.id }
        } else {
            Route::ViewBook { book_id: item.id }
        };

        html! {
            <div class="book-list-item">
                <Link<Route> to={ route_to } classes="poster">
                    { self.render_tools(ctx) }
                    <img src={ item.thumb_path.get_book_http_path().into_owned() } />
                    {
                        if is_updating {
                            html! {
                                <div class="changing"></div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </Link<Route>>

                {
                    if let Some(&(Progression::Ebook { chapter, .. }, ref file)) = ctx.props().progress.as_ref() {
                        html! {
                            <div class="progress" title={ format!("Reading Chapter {}/{}", chapter + 1, file.chapter_count) }>
                                <div class="prog-bar" style={ format!("width: {}%;", (chapter as f32 / file.chapter_count as f32 * 100.0) as i32) }></div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                <div class="info">
                    <div class="title" title={ item.title.clone() }><Link<Route> to={ Route::ViewBook { book_id: item.id } }>{ item.title.clone() }</Link<Route>></div>
                    {
                        if let Some(author) = item.cached.author.as_ref() {
                            html! {
                                <div class="author" title={ author.clone() }>{ author.clone() }</div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
            </div>
        }
    }
}

impl BookPosterItem {
    fn render_tools(&self, ctx: &Context<Self>) -> Html {
        let &BookPosterItemProps {
            is_editing,
            ref item,
            ref callback,
            ..
        } = ctx.props();

        if callback.is_none() {
            return html! {};
        }

        let book_id = item.id;

        let on_click_more = ctx.link().callback(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            let target = e.target_unchecked_into::<HtmlElement>();
            let bb = target.get_bounding_client_rect();

            BookPosterItemMsg::PosterItem(PosterItem::ShowPopup(DisplayOverlayItem::More {
                book_id,
                mouse_pos: (
                    (bb.left() + bb.width()) as i32,
                    (bb.top() + bb.height()) as i32,
                ),
            }))
        });

        html! {
            <>
                <div class="top-left">
                    <input
                        checked={ is_editing }
                        type="checkbox"
                        onclick={ ctx.link().callback(move |e: MouseEvent| {
                            e.prevent_default();
                            e.stop_propagation();

                            BookPosterItemMsg::Ignore
                        }) }
                        onmouseup={ ctx.link().callback(move |e: MouseEvent| {
                            let input = e.target_unchecked_into::<HtmlInputElement>();

                            let value = !input.checked();

                            input.set_checked(value);

                            BookPosterItemMsg::AddOrRemoveItemFromEditing(book_id, value)
                        }) }
                    />
                </div>
                <div class="bottom-right">
                    <span class="material-icons" onclick={ on_click_more } title="More Options">{ "more_horiz" }</span>
                </div>
                <div class="bottom-left">
                    <span class="material-icons" onclick={ ctx.link().callback_future(move |e: MouseEvent| {
                        e.prevent_default();
                        e.stop_propagation();

                        async move {
                            let resp = request::get_media_view(book_id).await;

                            match resp.ok() {
                                Ok(res) => BookPosterItemMsg::PosterItem(PosterItem::ShowPopup(DisplayOverlayItem::Edit(Box::new(res)))),
                                Err(err) => {
                                    crate::display_error(err);
                                    BookPosterItemMsg::Ignore
                                }
                            }

                        }
                    }) } title="More Options">{ "edit" }</span>
                </div>
            </>
        }
    }
}

#[derive(Clone)]
pub enum PosterItem {
    // Poster Specific Buttons
    ShowPopup(DisplayOverlayItem),

    // Popup Events
    UpdateBookById(BookId),

    UnMatch(BookId),

    UpdateBookByFiles(BookId),
}

#[derive(Clone)]
pub enum DisplayOverlayItem {
    Info {
        book_id: BookId,
    },

    Edit(Box<api::GetBookResponse>),

    More {
        book_id: BookId,
        mouse_pos: (i32, i32),
    },

    SearchForBook {
        book_id: BookId,
        input_value: Option<String>,
    },
}

impl PartialEq for DisplayOverlayItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Info { book_id: l_id }, Self::Info { book_id: r_id }) => l_id == r_id,
            (Self::More { book_id: l_id, .. }, Self::More { book_id: r_id, .. }) => l_id == r_id,
            (
                Self::SearchForBook {
                    book_id: l_id,
                    input_value: l_val,
                    ..
                },
                Self::SearchForBook {
                    book_id: r_id,
                    input_value: r_val,
                    ..
                },
            ) => l_id == r_id && l_val == r_val,

            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum DropdownInfoPopupEvent {
    Closed,
    RefreshMetadata,
    UnMatchBook,
    SearchFor,
    Info,
}

#[derive(Properties, PartialEq)]
pub struct DropdownInfoPopupProps {
    pub pos_x: i32,
    pub pos_y: i32,

    pub book_id: BookId,
    pub is_matched: bool,

    pub event: Callback<DropdownInfoPopupEvent>,
}

#[function_component(DropdownInfoPopup)]
pub fn _dropdown_info(props: &DropdownInfoPopupProps) -> Html {
    let book_id = props.book_id;

    html! {
        <Popup type_of={ PopupType::AtPoint(props.pos_x, props.pos_y) } on_close={ props.event.reform(|_| DropdownInfoPopupEvent::Closed) }>
            <div class="menu-list">
                // <PopupClose class="menu-item">{ "Start Reading" }</PopupClose>

                {
                    if props.is_matched {
                        html! {
                            <PopupClose class="menu-item" onclick={ on_click_prevdef_cb(
                                props.event.clone(),
                                |cb, _| cb.emit(DropdownInfoPopupEvent::UnMatchBook)
                            ) }>{ "Unmatch Book" }</PopupClose>
                        }
                    } else {
                        html! {}
                    }
                }

                <PopupClose class="menu-item" onclick={ on_click_prevdef_cb(
                    props.event.clone(),
                    |cb, _| cb.emit(DropdownInfoPopupEvent::RefreshMetadata)
                ) }>{ "Refresh Metadata" }</PopupClose>

                {
                    if cfg!(feature = "web") {
                        use gloo_utils::window;
                        use wasm_bindgen::UnwrapThrowExt;

                        html! {
                            <PopupClose class="menu-item" onclick={
                                Callback::from(move |_| {
                                    window().open_with_url_and_target(
                                        &request::get_download_path(Either::Left(book_id)),
                                        "_blank"
                                    ).unwrap_throw();
                                })
                            }>{ "Download" }</PopupClose>
                        }
                    } else {
                        html! {}
                    }
                }

                <PopupClose class="menu-item" onclick={ on_click_prevdef_stopprop_cb(
                    props.event.clone(),
                    |cb, _| cb.emit(DropdownInfoPopupEvent::SearchFor)
                ) }>{ "Search For Book" }</PopupClose>

                // <PopupClose class="menu-item">{ "Delete" }</PopupClose>

                <PopupClose class="menu-item" onclick={ on_click_prevdef_stopprop_cb(
                    props.event.clone(),
                    |cb, _| cb.emit(DropdownInfoPopupEvent::Info)
                ) }>{ "Show Info" }</PopupClose>
            </div>
        </Popup>
    }
}
