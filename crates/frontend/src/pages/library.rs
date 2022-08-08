use std::{rc::Rc, sync::Mutex, collections::{HashMap, HashSet}};

use common_local::{api, DisplayItem, ws::{WebsocketNotification, UniqueId, TaskType}, LibraryId};
use common::{BookId, component::popup::{Popup, PopupClose, PopupType}, PersonId};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{HtmlElement, UrlSearchParams, HtmlInputElement};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_router::prelude::Link;

use crate::{Route, request, components::{PopupSearchBook, PopupEditMetadata, MassSelectBar}, services::WsEventBus, util::{on_click_prevdef, on_click_prevdef_stopprop}};


#[derive(Properties, PartialEq)]
pub struct Property {
    pub library_id: LibraryId,
}

#[derive(Clone)]
pub enum Msg {
    HandleWebsocket(WebsocketNotification),

    // Requests
    RequestMediaItems,

    // Results
    MediaListResults(api::GetBookListResponse),
    BookItemResults(UniqueId, DisplayItem),

    // Events
    OnScroll(i32),
    PosterItem(PosterItem),
    ClosePopup,

    InitEventListenerAfterMediaItems,

    AddOrRemoveItemFromEditing(BookId, bool),
    DeselectAllEditing,

    Ignore
}

pub struct LibraryPage {
    on_scroll_fn: Option<Closure<dyn FnMut()>>,

    media_items: Option<Vec<DisplayItem>>,
    total_media_count: usize,

    is_fetching_media_items: bool,

    media_popup: Option<DisplayOverlay>,

    library_list_ref: NodeRef,

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
            on_scroll_fn: None,

            media_items: None,
            total_media_count: 0,

            is_fetching_media_items: false,

            media_popup: None,

            library_list_ref: NodeRef::default(),

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

                    WebsocketNotification::TaskEnd(id) => {
                        if let Some(book_id) = self.task_items.get(&id).copied() {
                            ctx.link()
                            .send_future(async move {
                                Msg::BookItemResults(id, request::get_media_view(book_id).await.book.into())
                            });
                        }
                    }
                }
            }

            Msg::BookItemResults(unique_id, book_item) => {
                if let Some(book_id) = self.task_items.remove(&unique_id) {
                    self.task_items_updating.remove(&book_id);
                }

                if let Some(items) = self.media_items.as_mut() {
                    if let Some(current_item) = items.iter_mut().find(|v| v.id == book_item.id) {
                        *current_item = book_item;
                    }
                }
            }

            Msg::ClosePopup => {
                self.media_popup = None;
            }

            Msg::DeselectAllEditing => {
                self.editing_items.lock().unwrap().clear();
            }

            Msg::AddOrRemoveItemFromEditing(id, value) => {
                let mut items = self.editing_items.lock().unwrap();

                if value {
                    if !items.iter().any(|v| *v == id) {
                        items.push(id);
                    }
                } else if let Some(index) = items.iter().position(|v| *v == id) {
                    items.swap_remove(index);
                }
            }

            Msg::InitEventListenerAfterMediaItems => {
                let lib_list_ref = self.library_list_ref.clone();
                let link = ctx.link().clone();

                let func = Closure::wrap(Box::new(move || {
                    let lib_list = lib_list_ref.cast::<HtmlElement>().unwrap();

                    link.send_message(Msg::OnScroll(lib_list.client_height() + lib_list.scroll_top()));
                }) as Box<dyn FnMut()>);

                let _ = self.library_list_ref.cast::<HtmlElement>().unwrap().add_event_listener_with_callback("scroll", func.as_ref().unchecked_ref());

                self.on_scroll_fn = Some(func);
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
                        get_search_query()
                    ).await)
                });
            }

            Msg::MediaListResults(mut resp) => {
                self.is_fetching_media_items = false;
                self.total_media_count = resp.count;

                if let Some(items) = self.media_items.as_mut() {
                    items.append(&mut resp.items);
                } else {
                    self.media_items = Some(resp.items);
                }
            }

            Msg::OnScroll(scroll_y) => {
                let scroll_height = self.library_list_ref.cast::<HtmlElement>().unwrap().scroll_height();

                if scroll_height - scroll_y < 600 && self.can_req_more() {
                    ctx.link().send_message(Msg::RequestMediaItems);
                }
            }

            Msg::PosterItem(item) => match item {
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

                PosterItem::UpdateBookBySource(book_id) => {
                    ctx.link()
                    .send_future(async move {
                        request::update_book(book_id, &api::PostBookBody::AutoMatchBookIdBySource).await;

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

            Msg::Ignore => return false,
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
        if self.on_scroll_fn.is_none() && self.library_list_ref.get().is_some() {
            ctx.link().send_message(Msg::InitEventListenerAfterMediaItems);
        } else if first_render {
            ctx.link().send_message(Msg::RequestMediaItems);
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        // TODO: Determine if still needed.
        if let Some(f) = self.on_scroll_fn.take() {
            let _ = self.library_list_ref.cast::<HtmlElement>().unwrap().remove_event_listener_with_callback("scroll", f.as_ref().unchecked_ref());
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
                    <div class="book-list normal" ref={ self.library_list_ref.clone() }>
                        {
                            for items.iter().map(|item| {
                                let is_editing = self.editing_items.lock().unwrap().contains(&item.id);
                                let is_updating = self.task_items_updating.contains(&item.id);

                                html! {
                                    <MediaItem
                                        {is_editing}
                                        {is_updating}

                                        item={item.clone()}
                                        callback={ctx.link().callback(|v| v)}
                                        library_list_ref={self.library_list_ref.clone()}
                                    />
                                }
                            })
                        }
                        // { for (0..remaining).map(|_| Self::render_placeholder_item()) }

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
                                                        on_click_prevdef(ctx.link(), Msg::PosterItem(PosterItem::UpdateBookBySource(book_id)))
                                                    }>{ "Refresh Metadata" }</PopupClose>
                                                    <PopupClose class="menu-item" onclick={
                                                        on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::SearchForBook { book_id, input_value: None })))
                                                    }>{ "Search For Book" }</PopupClose>
                                                    <PopupClose class="menu-item" onclick={
                                                        on_click_prevdef(ctx.link(), Msg::PosterItem(PosterItem::UpdateBookByFiles(book_id)))
                                                    }>{ "Quick Search By Files" }</PopupClose>
                                                    <PopupClose class="menu-item" >{ "Delete" }</PopupClose>
                                                    <PopupClose class="menu-item" onclick={
                                                        on_click_prevdef_stopprop(ctx.link(), Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::Info { book_id })))
                                                    }>{ "Show Info" }</PopupClose>
                                                </div>
                                            </Popup>
                                        }
                                    }

                                    DisplayOverlay::Edit(resp) => {
                                        html! {
                                            <PopupEditMetadata
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
                    </div>

                    <MassSelectBar
                        on_deselect_all={ctx.link().callback(|_| Msg::DeselectAllEditing)}
                        editing_container={self.library_list_ref.clone()}
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



// Media Item

#[derive(Properties)]
pub struct MediaItemProps {
    pub item: DisplayItem,
    pub callback: Option<Callback<Msg>>,
    pub library_list_ref: Option<NodeRef>,
    #[prop_or_default]
    pub is_editing: bool,
    #[prop_or_default]
    pub is_updating: bool,
}

impl PartialEq for MediaItemProps {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item &&
        self.library_list_ref == other.library_list_ref &&
        self.is_editing == other.is_editing &&
        self.is_updating == other.is_updating
    }
}


pub struct MediaItem;

impl Component for MediaItem {
    type Message = Msg;
    type Properties = MediaItemProps;

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
        let &MediaItemProps {
            is_editing,
            is_updating,
            ref item,
            ref library_list_ref,
            ..
        } = ctx.props();

        let library_list_ref_clone = library_list_ref.clone();

        let book_id = item.id;

        let on_click_more = ctx.link().callback(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            let scroll = library_list_ref_clone.as_ref().unwrap().cast::<HtmlElement>().unwrap().scroll_top();

            Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::More { book_id, mouse_pos: (e.page_x(), e.page_y() + scroll) }))
        });

        html! {
            <Link<Route> to={Route::ViewBook { book_id: item.id }} classes={ classes!("book-list-item") }>
                <div class="poster">
                    <div class="top-left">
                        <input
                            checked={is_editing}
                            type="checkbox"
                            onclick={ctx.link().callback(move |e: MouseEvent| {
                                e.prevent_default();
                                e.stop_propagation();

                                Msg::Ignore
                            })}
                            onmouseup={ctx.link().callback(move |e: MouseEvent| {
                                let input = e.target_unchecked_into::<HtmlInputElement>();

                                let value = !input.checked();

                                input.set_checked(value);

                                Msg::AddOrRemoveItemFromEditing(book_id, value)
                            })}
                        />
                    </div>
                    <div class="bottom-right">
                        {
                            if library_list_ref.is_some() {
                                html! {
                                    <span class="material-icons" onclick={on_click_more} title="More Options">{ "more_horiz" }</span>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                    <div class="bottom-left">
                        <span class="material-icons" onclick={ctx.link().callback_future(move |e: MouseEvent| {
                            e.prevent_default();
                            e.stop_propagation();

                            async move {
                                Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::Edit(Box::new(request::get_media_view(book_id).await))))
                            }
                        })} title="More Options">{ "edit" }</span>
                    </div>
                    <img src={ item.get_thumb_url() } />
                    {
                        if is_updating {
                            html! {
                                <div class="changing"></div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
                <div class="info">
                    <div class="title" title={ item.title.clone() }>{ item.title.clone() }</div>
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
            </Link<Route>>
        }
    }
}




#[derive(Clone)]
pub enum PosterItem {
    // Poster Specific Buttons
    ShowPopup(DisplayOverlay),

    // Popup Events
    UpdateBookBySource(BookId),

    // Popup Events
    UpdateBookByFiles(BookId),
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

fn get_search_query() -> Option<api::SearchQuery> {
    let search_params = UrlSearchParams::new_with_str(
        &gloo_utils::window().location().search().ok()?
    ).ok()?;

    let query = search_params.get("query");
    let source = search_params.get("source");
    let person_id = search_params.get("person_id");

    if query.is_none() && source.is_none() {
        None
    } else {
        Some(api::SearchQuery {
            query,
            source,
            person_id: person_id.and_then(|v| Some(PersonId::from(v.parse::<usize>().ok()?)))
        })
    }
}