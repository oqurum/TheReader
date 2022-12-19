use std::{cell::RefCell, rc::Rc};

use common::{
    component::{Popup, PopupClose, PopupType},
    BookId, Either,
};
use common_local::{api, CollectionId, DisplayItem, MediaItem, Progression, ThumbnailStoreExt};
use web_sys::{HtmlElement, HtmlInputElement, MouseEvent};
use yew::{
    classes, function_component, html, use_context, Callback, Component, Context, Html, Properties,
    TargetCast,
};
use yew_hooks::use_async;
use yew_router::prelude::Link;

use crate::{
    components::{PopupEditBook, PopupSearchBook},
    request,
    util::{on_click_prevdef_cb, on_click_prevdef_stopprop_cb},
    BaseRoute,
};

use super::{BookListScope, OwnerBarrier};

#[derive(Properties)]
pub struct BookPosterItemProps {
    // TODO: Convert to Either<DisplayItem, BookProgression> and remove progress field.
    pub item: DisplayItem,
    pub callback: Option<Callback<BookPosterItemMsg>>,
    pub editing_items: Option<Rc<RefCell<Vec<BookId>>>>,

    // i64 is currently just total chapter count
    pub progress: Option<(Progression, MediaItem)>,

    #[prop_or_default]
    pub is_editing: bool,
    #[prop_or_default]
    pub is_updating: bool,
    #[prop_or_default]
    pub disable_tools: bool,
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
    ShowPopup(DisplayOverlayItem),

    // Popup Events
    UpdateBookById(BookId),
    AddBookToCollection(BookId, CollectionId),
    RemoveBookFromCollection(BookId, CollectionId),

    UnMatch(BookId),
    MarkAsRead,
    MarkAsUnread,

    // Other
    AddOrRemoveItemFromEditing(BookId, bool),

    ClosePopup,

    Ignore,
}

pub struct BookPosterItem {
    media_popup: Option<DisplayOverlayItem>,
}

impl Component for BookPosterItem {
    type Message = BookPosterItemMsg;
    type Properties = BookPosterItemProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self { media_popup: None }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg.clone() {
            BookPosterItemMsg::ClosePopup => {
                self.media_popup = None;
            }

            BookPosterItemMsg::AddOrRemoveItemFromEditing(id, value) => {
                if let Some(items) = ctx.props().editing_items.as_ref() {
                    let mut items = items.borrow_mut();

                    if value {
                        if !items.iter().any(|v| *v == id) {
                            items.push(id);
                        }
                    } else if let Some(index) = items.iter().position(|v| *v == id) {
                        items.swap_remove(index);
                    }
                }
            }

            BookPosterItemMsg::UpdateBookById(book_id) => {
                ctx.link().send_future(async move {
                    request::update_book(book_id, &api::PostBookBody::AutoMatchBookId).await;

                    BookPosterItemMsg::Ignore
                });
            }

            BookPosterItemMsg::AddBookToCollection(book_id, collection_id) => {
                ctx.link().send_future(async move {
                    request::add_book_to_collection(collection_id, book_id).await;

                    BookPosterItemMsg::Ignore
                });
            }

            BookPosterItemMsg::RemoveBookFromCollection(book_id, collection_id) => {
                ctx.link().send_future(async move {
                    request::remove_book_from_collection(collection_id, book_id).await;

                    BookPosterItemMsg::Ignore
                });
            }

            BookPosterItemMsg::UnMatch(book_id) => {
                ctx.link().send_future(async move {
                    request::update_book(book_id, &api::PostBookBody::UnMatch).await;

                    BookPosterItemMsg::Ignore
                });
            }

            BookPosterItemMsg::MarkAsRead => {
                let file_id = ctx.props().progress.as_ref().unwrap().1.id;

                ctx.link().send_future(async move {
                    request::update_book_progress(file_id, &Progression::Complete).await;

                    BookPosterItemMsg::Ignore
                });
            }

            BookPosterItemMsg::MarkAsUnread => {
                let file_id = ctx.props().progress.as_ref().unwrap().1.id;

                ctx.link().send_future(async move {
                    request::remove_book_progress(file_id).await;

                    BookPosterItemMsg::Ignore
                });
            }

            BookPosterItemMsg::ShowPopup(new_disp) => {
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

            BookPosterItemMsg::Ignore => (),
        }

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
            BaseRoute::ReadBook { book_id: file.id }
        } else {
            BaseRoute::ViewBook { book_id: item.id }
        };

        html! {
            <div class="book-list-item">
                <Link<BaseRoute> to={ route_to } classes="poster link-light">
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
                </Link<BaseRoute>>

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
                    <div title={ item.title.clone() } class="title-container">
                        <Link<BaseRoute> classes="title link-light" to={ BaseRoute::ViewBook { book_id: item.id } }>{ item.title.clone() }</Link<BaseRoute>>
                    </div>
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

                {
                    if let Some(overlay_type) = self.media_popup.as_ref() {
                        match overlay_type {
                            DisplayOverlayItem::Info { book_id: _ } => {
                                html! {
                                    <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| BookPosterItemMsg::ClosePopup)}>
                                        <div class="modal-header">
                                            <h5 class="modal-title">{ "Info" }</h5>
                                        </div>
                                        <div class="modal-body">
                                            <span>{ "TODO" }</span>
                                        </div>
                                    </Popup>
                                }
                            }

                            &DisplayOverlayItem::More { book_id, mouse_pos: (pos_x, pos_y) } => {
                                let is_matched = true;//items.iter().any(|b| b.source.agent.as_ref() == "local");

                                html! {
                                    <DropdownInfoPopup
                                        { pos_x }
                                        { pos_y }

                                        { book_id }
                                        { is_matched }
                                        progress={ ctx.props().progress.as_ref().map(|v| v.0) }

                                        event={ ctx.link().callback(move |e| {
                                            log::debug!("{e:?}");

                                            match e {
                                                DropdownInfoPopupEvent::Closed => BookPosterItemMsg::ClosePopup,
                                                DropdownInfoPopupEvent::UnMatchBook => BookPosterItemMsg::UnMatch(book_id),
                                                DropdownInfoPopupEvent::AddToCollection(id) => BookPosterItemMsg::AddBookToCollection(book_id, id),
                                                DropdownInfoPopupEvent::RemoveFromCollection(id) => BookPosterItemMsg::RemoveBookFromCollection(book_id, id),
                                                DropdownInfoPopupEvent::RefreshMetadata => BookPosterItemMsg::UpdateBookById(book_id),
                                                DropdownInfoPopupEvent::SearchFor => BookPosterItemMsg::ShowPopup(DisplayOverlayItem::SearchForBook { book_id, input_value: None }),
                                                DropdownInfoPopupEvent::Info => BookPosterItemMsg::ShowPopup(DisplayOverlayItem::Info { book_id }),
                                                DropdownInfoPopupEvent::MarkAsRead => BookPosterItemMsg::MarkAsRead,
                                                DropdownInfoPopupEvent::MarkAsUnread => BookPosterItemMsg::MarkAsUnread,
                                            }
                                        }) }
                                    />
                                }
                            }

                            DisplayOverlayItem::Edit(resp) => {
                                html! {
                                    <PopupEditBook
                                        on_close={ ctx.link().callback(|_| BookPosterItemMsg::ClosePopup) }
                                        classes={ classes!("popup-book-edit") }
                                        media_resp={ (**resp).clone() }
                                    />
                                }
                            }

                            &DisplayOverlayItem::SearchForBook { book_id, ref input_value } => {
                                let input_value = if let Some(v) = input_value {
                                    v.to_string()
                                } else {
                                    let item = &ctx.props().item;

                                    format!("{} {}", item.title.clone(), item.cached.author.as_deref().unwrap_or_default())
                                };

                                let input_value = input_value.trim().to_string();

                                html! {
                                    <PopupSearchBook {book_id} {input_value} on_close={ ctx.link().callback(|_| BookPosterItemMsg::ClosePopup) } />
                                }
                            }
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
        }
    }
}

impl BookPosterItem {
    fn render_tools(&self, ctx: &Context<Self>) -> Html {
        let &BookPosterItemProps {
            disable_tools,
            is_editing,
            ref item,
            ..
        } = ctx.props();

        if disable_tools {
            return html! {};
        }

        let book_id = item.id;

        let on_click_more = ctx.link().callback(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            let target = e.target_unchecked_into::<HtmlElement>();
            let bb = target.get_bounding_client_rect();

            BookPosterItemMsg::ShowPopup(DisplayOverlayItem::More {
                book_id,
                mouse_pos: (
                    (bb.left() + bb.width()) as i32,
                    (bb.top() + bb.height()) as i32,
                ),
            })
        });

        let on_click_edit = ctx.link().callback_future(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            async move {
                let resp = request::get_media_view(book_id).await;

                match resp.ok() {
                    Ok(res) => BookPosterItemMsg::ShowPopup(DisplayOverlayItem::Edit(Box::new(res))),
                    Err(err) => {
                        crate::display_error(err);
                        BookPosterItemMsg::Ignore
                    }
                }
            }
        });

        html! {
            <>
                <OwnerBarrier>
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
                </OwnerBarrier>

                <div class="bottom-right">
                    <span class="material-icons" onclick={ on_click_more } title="More Options">{ "more_horiz" }</span>
                </div>

                <OwnerBarrier>
                    <div class="bottom-left">
                        <span class="material-icons" onclick={ on_click_edit } title="More Options">{ "edit" }</span>
                    </div>
                </OwnerBarrier>
            </>
        }
    }
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

#[derive(Debug, Clone, Copy)]
pub enum DropdownInfoPopupEvent {
    Closed,
    RefreshMetadata,
    AddToCollection(CollectionId),
    RemoveFromCollection(CollectionId),
    MarkAsUnread,
    MarkAsRead,
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
    pub progress: Option<Progression>,

    pub event: Callback<DropdownInfoPopupEvent>,
}

#[function_component(DropdownInfoPopup)]
pub fn _dropdown_info(props: &DropdownInfoPopupProps) -> Html {
    let book_id = props.book_id;

    let book_list_ctx = use_context::<BookListScope>().unwrap_or_default();

    let display_collections = use_async(async move { request::get_collections().await.ok() });

    let add_to_collection_cb = {
        let display_collections = display_collections.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            display_collections.run();
        })
    };

    if display_collections.loading
        || display_collections.data.is_some()
        || display_collections.error.is_some()
    {
        return html! {
            <Popup type_of={ PopupType::AtPoint(props.pos_x, props.pos_y) } on_close={ props.event.reform(|_| DropdownInfoPopupEvent::Closed) }>
                {
                    if display_collections.loading {
                        html! {
                            <div class="dropdown-menu dropdown-menu-dark show">
                                <div class="dropdown-item"></div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                {
                    if let Some(data) = display_collections.data.as_ref() {
                        html! {
                            <div class="dropdown-menu dropdown-menu-dark show">
                                {
                                    for data.iter().map(|d| {
                                        let id = d.id;

                                        html! {
                                            <PopupClose
                                                class="dropdown-item"
                                                onclick={ on_click_prevdef_cb(
                                                    props.event.clone(),
                                                    move |cb, _| cb.emit(DropdownInfoPopupEvent::AddToCollection(id))
                                                ) }
                                            >{ d.name.clone() }</PopupClose>
                                        }
                                    })
                                }
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                {
                    if let Some(data) = display_collections.error.as_ref() {
                        html! {
                            <div class="dropdown-menu dropdown-menu-dark show">
                                <div class="dropdown-item">{ data.description.clone() }</div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
            </Popup>
        };
    }

    html! {
        <Popup type_of={ PopupType::AtPoint(props.pos_x, props.pos_y) } on_close={ props.event.reform(|_| DropdownInfoPopupEvent::Closed) }>
            <div class="dropdown-menu dropdown-menu-dark show">
                // <PopupClose class="menu-item">{ "Start Reading" }</PopupClose>

                {
                    if props.progress != Some(Progression::Complete) {
                        html! {
                            <>
                                <PopupClose class="dropdown-item" onclick={ on_click_prevdef_cb(
                                    props.event.clone(),
                                    |cb, _| cb.emit(DropdownInfoPopupEvent::MarkAsRead)
                                ) }>{ "Mark As Read" }</PopupClose>
                            </>
                        }
                    } else {
                        html! {}
                    }
                }

                {
                    if props.progress.is_some() {
                        html! {
                            <>
                                <PopupClose class="dropdown-item" onclick={ on_click_prevdef_cb(
                                    props.event.clone(),
                                    |cb, _| cb.emit(DropdownInfoPopupEvent::MarkAsUnread)
                                ) }>{ "Mark As Unread" }</PopupClose>
                            </>
                        }
                    } else {
                        html! {}
                    }
                }

                <OwnerBarrier>
                {
                    if props.is_matched {
                        html! {
                            <PopupClose class="dropdown-item" onclick={ on_click_prevdef_cb(
                                props.event.clone(),
                                |cb, _| cb.emit(DropdownInfoPopupEvent::UnMatchBook)
                            ) }>{ "Unmatch Book" }</PopupClose>
                        }
                    } else {
                        html! {}
                    }
                }
                </OwnerBarrier>

                <OwnerBarrier>
                    <PopupClose class="dropdown-item" onclick={ on_click_prevdef_cb(
                        props.event.clone(),
                        |cb, _| cb.emit(DropdownInfoPopupEvent::RefreshMetadata)
                    ) }>{ "Refresh Metadata" }</PopupClose>
                </OwnerBarrier>

                {
                    if let Some(id) = book_list_ctx.collection_id {
                        html! {
                            <PopupClose
                                class="dropdown-item"
                                onclick={ on_click_prevdef_cb(
                                    props.event.clone(),
                                    move |cb, _| cb.emit(DropdownInfoPopupEvent::RemoveFromCollection(id))
                                ) }
                            >{ "Remove from Collection" }</PopupClose>
                        }
                    } else {
                        html! {
                            <div class="dropdown-item" onclick={ add_to_collection_cb }>{ "Add to Collection" }</div>
                        }
                    }
                }

                {
                    if cfg!(feature = "web") {
                        use gloo_utils::window;
                        use wasm_bindgen::UnwrapThrowExt;

                        html! {
                            <PopupClose class="dropdown-item" onclick={
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

                <OwnerBarrier>
                    <PopupClose class="dropdown-item" onclick={ on_click_prevdef_stopprop_cb(
                        props.event.clone(),
                        |cb, _| cb.emit(DropdownInfoPopupEvent::SearchFor)
                    ) }>{ "Search For Book" }</PopupClose>
                </OwnerBarrier>

                // <PopupClose class="dropdown-item">{ "Delete" }</PopupClose>

                <PopupClose class="dropdown-item" onclick={ on_click_prevdef_stopprop_cb(
                    props.event.clone(),
                    |cb, _| cb.emit(DropdownInfoPopupEvent::Info)
                ) }>{ "Show Info" }</PopupClose>
            </div>
        </Popup>
    }
}
