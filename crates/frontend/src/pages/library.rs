use std::{rc::Rc, sync::Mutex, collections::{HashMap, HashSet}};

use common_local::{api, DisplayItem, ws::{WebsocketNotification, UniqueId, TaskType}, LibraryId};
use common::{BookId, component::{InfiniteScroll, InfiniteScrollEvent, Popup, PopupClose, PopupType}, api::WrappingResponse};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{request, components::{DropdownInfoPopup, DropdownInfoPopupEvent, PopupSearchBook, PopupEditBook, MassSelectBar, Sidebar, book_poster_item::{BookPosterItem, DisplayOverlayItem, PosterItem, BookPosterItemMsg}}, services::WsEventBus, util::{on_click_prevdef, on_click_prevdef_stopprop, SearchQuery}};


#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub library_id: LibraryId,
}

#[derive(Clone)]
pub enum Msg {
    HandleWebsocket(WebsocketNotification),

    // Requests
    RequestMediaItems,

    // Results
    MediaListResults(WrappingResponse<api::GetBookListResponse>),
    BookItemResults(UniqueId, WrappingResponse<DisplayItem>),

    // Events
    OnScroll(InfiniteScrollEvent),
    ClosePopup,

    DeselectAllEditing,

    BookListItemEvent(BookPosterItemMsg),

    Ignore
}

pub struct LibraryPage {
    media_items: Option<Vec<DisplayItem>>,
    total_media_count: usize,

    is_fetching_media_items: bool,

    media_popup: Option<DisplayOverlayItem>,

    book_list_ref: NodeRef,

    // TODO: Make More Advanced
    editing_items: Rc<Mutex<Vec<BookId>>>,

    _producer: Box<dyn Bridge<WsEventBus>>,

    // TODO: I should just have a global one
    task_items: HashMap<UniqueId, BookId>,
    // Used along with task_items
    task_items_updating: HashSet<BookId>,
}

impl Component for LibraryPage {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            media_items: None,
            total_media_count: 0,

            is_fetching_media_items: false,

            media_popup: None,

            book_list_ref: NodeRef::default(),

            editing_items: Rc::new(Mutex::new(Vec::new())),

            _producer: WsEventBus::bridge(ctx.link().callback(Msg::HandleWebsocket)),

            task_items: HashMap::new(),
            task_items_updating: HashSet::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleWebsocket(value) => {
                match value {
                    WebsocketNotification::TaskStart { id, type_of } => {
                        if let TaskType::UpdatingBook(book_id) = type_of {
                            self.task_items.insert(id, book_id);
                            self.task_items_updating.insert(book_id);
                        }
                    }

                    WebsocketNotification::TaskTypeEnd { id, type_of } => {
                        if let TaskType::UpdatingBook(book_id) = type_of {
                            self.task_items_updating.remove(&book_id);
                            self.task_items.remove(&id); // TODO: I shouldn't remove this.

                            if let Some(items) = self.media_items.as_ref() {
                                if items.iter().any(|v| v.id == book_id) {
                                    ctx.link()
                                    .send_future(async move {
                                        Msg::BookItemResults(id, request::get_media_view(book_id).await.map(|v| v.book.into()))
                                    });
                                }
                            }
                        }
                    }

                    WebsocketNotification::TaskEnd(id) => {
                        if let Some(book_id) = self.task_items.get(&id).copied() {
                            self.task_items_updating.remove(&book_id);
                            self.task_items.remove(&id);

                            if let Some(items) = self.media_items.as_ref() {
                                if items.iter().any(|v| v.id == book_id) {
                                    ctx.link()
                                    .send_future(async move {
                                        Msg::BookItemResults(id, request::get_media_view(book_id).await.map(|v| v.book.into()))
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Msg::BookItemResults(unique_id, resp) => {
                match resp.ok() {
                    Ok(book_item) => {
                        if let Some(book_id) = self.task_items.remove(&unique_id) {
                            self.task_items_updating.remove(&book_id);
                        }

                        if let Some(items) = self.media_items.as_mut() {
                            if let Some(current_item) = items.iter_mut().find(|v| v.id == book_item.id) {
                                *current_item = book_item;
                            }
                        }
                    }

                    Err(err) => crate::display_error(err),
                }
            }

            Msg::ClosePopup => {
                self.media_popup = None;
            }

            Msg::DeselectAllEditing => {
                self.editing_items.lock().unwrap().clear();
            }

            Msg::RequestMediaItems => {
                if self.is_fetching_media_items {
                    return false;
                }

                self.is_fetching_media_items = true;

                let offset = Some(self.media_items.as_ref().map(|v| v.len()).unwrap_or_default()).filter(|v| *v != 0);

                let library = ctx.props().library_id;

                ctx.link()
                .send_future(async move {
                    Msg::MediaListResults(request::get_books(
                        Some(library),
                        offset,
                        None,
                        SearchQuery::load().filters
                    ).await)
                });
            }

            Msg::MediaListResults(resp) => {
                self.is_fetching_media_items = false;

                match resp.ok() {
                    Ok(mut resp) => {
                        self.total_media_count = resp.count;

                        if let Some(items) = self.media_items.as_mut() {
                            items.append(&mut resp.items);
                        } else {
                            self.media_items = Some(resp.items);
                        }
                    }

                    Err(err) => crate::display_error(err),
                }

            }

            Msg::OnScroll(event) => {
                if event.scroll_height - event.scroll_pos < 600 && self.can_req_more() {
                    ctx.link().send_message(Msg::RequestMediaItems);
                }

                return false;
            }

            Msg::BookListItemEvent(event) => {
                match event {
                    BookPosterItemMsg::AddOrRemoveItemFromEditing(id, value) => {
                        let mut items = self.editing_items.lock().unwrap();

                        if value {
                            if !items.iter().any(|v| *v == id) {
                                items.push(id);
                            }
                        } else if let Some(index) = items.iter().position(|v| *v == id) {
                            items.swap_remove(index);
                        }
                    }

                    BookPosterItemMsg::PosterItem(item) => match item {
                        PosterItem::ShowPopup(new_disp) => {
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

                        PosterItem::UpdateBookById(book_id) => {
                            ctx.link()
                            .send_future(async move {
                                request::update_book(book_id, &api::PostBookBody::AutoMatchBookId).await;

                                Msg::Ignore
                            });
                        }

                        PosterItem::UpdateBookByFiles(book_id) => {
                            ctx.link()
                            .send_future(async move {
                                request::update_book(book_id, &api::PostBookBody::AutoMatchBookIdByFiles).await;

                                Msg::Ignore
                            });
                        }
                    }

                    BookPosterItemMsg::Ignore => (),
                }
            }

            Msg::Ignore => return false,
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <Sidebar />
                <div class="view-container">
                    { self.render_main(ctx) }
                </div>
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::RequestMediaItems);
        }
    }
}

impl LibraryPage {
    fn render_main(&self, ctx: &Context<Self>) -> Html {
        if let Some(items) = self.media_items.as_deref() {
            // TODO: Placeholders
            // let remaining = (self.total_media_count as usize - items.len()).min(50);

            html! {
                <>
                    <InfiniteScroll
                        ref={ self.book_list_ref.clone() }
                        class="book-list normal"
                        event={ ctx.link().callback(Msg::OnScroll) }
                    >
                        {
                            for items.iter().map(|item| {
                                let is_editing = self.editing_items.lock().unwrap().contains(&item.id);
                                let is_updating = self.task_items_updating.contains(&item.id);

                                html! {
                                    <BookPosterItem
                                        {is_editing}
                                        {is_updating}

                                        item={item.clone()}
                                        callback={ctx.link().callback(Msg::BookListItemEvent)}
                                    />
                                }
                            })
                        }
                        // { for (0..remaining).map(|_| Self::render_placeholder_item()) }

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
                                        html! {
                                            <DropdownInfoPopup
                                                { pos_x }
                                                { pos_y }
                                                { book_id }

                                                event={ ctx.link().callback(move |e| {
                                                    match e {
                                                        DropdownInfoPopupEvent::Closed => Msg::ClosePopup,
                                                        DropdownInfoPopupEvent::RefreshMetadata => Msg::BookListItemEvent(BookPosterItemMsg::PosterItem(PosterItem::UpdateBookById(book_id))),
                                                        DropdownInfoPopupEvent::SearchFor => Msg::BookListItemEvent(BookPosterItemMsg::PosterItem(PosterItem::ShowPopup(DisplayOverlayItem::SearchForBook { book_id, input_value: None }))),
                                                        DropdownInfoPopupEvent::Info => Msg::BookListItemEvent(BookPosterItemMsg::PosterItem(PosterItem::ShowPopup(DisplayOverlayItem::Info { book_id }))),
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
                                            let items = self.media_items.as_ref().unwrap();
                                            let item = items.iter().find(|v| v.id == book_id).unwrap();

                                            format!("{} {}", item.title.clone(), item.cached.author.as_deref().unwrap_or_default())
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
                    </InfiniteScroll>

                    <MassSelectBar
                        on_deselect_all={ctx.link().callback(|_| Msg::DeselectAllEditing)}
                        editing_container={self.book_list_ref.clone()}
                        editing_items={self.editing_items.clone()}
                    />
                </>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }


    // fn render_placeholder_item() -> Html {
    //     html! {
    //         <div class="book-list-item placeholder">
    //             <div class="poster"></div>
    //             <div class="info">
    //                 <a class="author"></a>
    //                 <a class="title"></a>
    //             </div>
    //         </div>
    //     }
    // }

    pub fn can_req_more(&self) -> bool {
        let count = self.media_items.as_ref().map(|v| v.len()).unwrap_or_default();

        count != 0 && count != self.total_media_count as usize
    }
}