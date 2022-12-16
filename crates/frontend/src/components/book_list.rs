use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use common::{
    api::WrappingResponse,
    component::{InfiniteScroll, InfiniteScrollEvent},
    BookId,
};
use common_local::{
    api,
    ws::{TaskId, TaskType, WebsocketNotification},
    CollectionId, DisplayItem,
};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{request, services::WsEventBus};

use super::book_poster_item::{BookPosterItem, BookPosterItemMsg};
use super::MassSelectBar;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BookListScope {
    pub collection_id: Option<CollectionId>,
}

pub struct BookListRequest {
    pub offset: Option<usize>,
    pub response: Callback<WrappingResponse<api::GetBookListResponse>>,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub on_load: Callback<BookListRequest>,
}

pub enum Msg {
    HandleWebsocket(WebsocketNotification),

    // Requests
    RequestMediaItems,

    // Results
    MediaListResults(WrappingResponse<api::GetBookListResponse>),
    BookItemResults(TaskId, WrappingResponse<DisplayItem>),

    // Events
    OnScroll(InfiniteScrollEvent),

    DeselectAllEditing,

    BookListItemEvent(BookPosterItemMsg),
}

pub struct BookListComponent {
    media_items: Option<Vec<DisplayItem>>,
    total_media_count: usize,

    is_fetching_media_items: bool,

    book_list_ref: NodeRef,

    // TODO: Make More Advanced
    editing_items: Rc<RefCell<Vec<BookId>>>,

    _producer: Box<dyn Bridge<WsEventBus>>,

    // TODO: I should just have a global one
    task_items: HashMap<TaskId, BookId>,
    // Used along with task_items
    task_items_updating: HashSet<BookId>,
}

impl Component for BookListComponent {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            media_items: None,
            total_media_count: 0,

            is_fetching_media_items: false,

            book_list_ref: NodeRef::default(),

            editing_items: Rc::new(RefCell::new(Vec::new())),

            _producer: {
                let cb = {
                    let link = ctx.link().clone();
                    move |e| link.send_message(Msg::HandleWebsocket(e))
                };

                WsEventBus::bridge(Rc::new(cb))
            },

            task_items: HashMap::new(),
            task_items_updating: HashSet::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleWebsocket(value) => {
                match value {
                    WebsocketNotification::TaskStart { .. } => {
                        //
                    }

                    WebsocketNotification::TaskUpdate {
                        id: task_id,
                        type_of,
                        inserting,
                    } => {
                        if let TaskType::UpdatingBook { id: book_id, .. } = type_of {
                            if inserting {
                                self.task_items.insert(task_id, book_id);
                                self.task_items_updating.insert(book_id);
                            } else {
                                self.task_items_updating.remove(&book_id);
                                self.task_items.remove(&task_id);
                            }

                            if let Some(items) = self.media_items.as_ref() {
                                if items.iter().any(|v| v.id == book_id) {
                                    ctx.link().send_future(async move {
                                        Msg::BookItemResults(
                                            task_id,
                                            request::get_media_view(book_id)
                                                .await
                                                .map(|v| v.book.into()),
                                        )
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
                                    ctx.link().send_future(async move {
                                        Msg::BookItemResults(
                                            id,
                                            request::get_media_view(book_id)
                                                .await
                                                .map(|v| v.book.into()),
                                        )
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Msg::BookItemResults(unique_id, resp) => match resp.ok() {
                Ok(book_item) => {
                    if let Some(book_id) = self.task_items.remove(&unique_id) {
                        self.task_items_updating.remove(&book_id);
                    }

                    if let Some(items) = self.media_items.as_mut() {
                        if let Some(current_item) = items.iter_mut().find(|v| v.id == book_item.id)
                        {
                            *current_item = book_item;
                        }
                    }
                }

                Err(err) => crate::display_error(err),
            },

            Msg::DeselectAllEditing => {
                self.editing_items.borrow_mut().clear();
            }

            Msg::RequestMediaItems => {
                if self.is_fetching_media_items {
                    return false;
                }

                self.is_fetching_media_items = true;

                let offset = Some(
                    self.media_items
                        .as_ref()
                        .map(|v| v.len())
                        .unwrap_or_default(),
                )
                .filter(|v| *v != 0);

                ctx.props().on_load.emit(BookListRequest {
                    offset,
                    response: ctx.link().callback(Msg::MediaListResults),
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

            Msg::BookListItemEvent(BookPosterItemMsg::RemoveBookFromCollection(
                book_id,
                _collection_id,
            )) => {
                // TODO: Ensure this is only called from inside a collection.
                let books = self.media_items.as_mut().unwrap();

                if let Some(index) = books.iter().position(|v| v.id == book_id) {
                    books.remove(index);
                    self.total_media_count -= 1;
                }
            }

            _ => (),
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if let Some(items) = self.media_items.as_deref() {
            // TODO: Placeholders
            // let remaining = (self.total_media_count as usize - items.len()).min(50);

            html! {
                <>
                    <InfiniteScroll
                        r#ref={ self.book_list_ref.clone() }
                        class="book-list normal"
                        event={ ctx.link().callback(Msg::OnScroll) }
                    >
                        {
                            for items.iter().map(|item| {
                                let is_editing = self.editing_items.borrow().contains(&item.id);
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

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::RequestMediaItems);
        }
    }
}

impl BookListComponent {
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
        let count = self
            .media_items
            .as_ref()
            .map(|v| v.len())
            .unwrap_or_default();

        count != 0 && count != self.total_media_count as usize
    }
}
