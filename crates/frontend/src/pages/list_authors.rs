use common::{
    api::WrappingResponse,
    component::{InfiniteScroll, InfiniteScrollEvent, Popup, PopupClose, PopupType},
    util::truncate_on_indices,
    PersonId,
};
use common_local::{api, Person, SearchType};
use gloo_utils::document;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement};
use yew::{html::Scope, prelude::*};
use yew_router::prelude::Link;

use crate::{
    request,
    util::{on_click_prevdef_scope, on_click_prevdef_stopprop_scope},
    BaseRoute,
};

#[derive(Clone)]
pub enum Msg {
    // Requests
    RequestPeople,

    // Results
    PeopleListResults(WrappingResponse<api::GetPeopleResponse>),
    PersonUpdateSearchResults(String, WrappingResponse<api::BookSearchResponse>),
    PersonCombineSearchResults(String, WrappingResponse<Vec<Person>>),

    // Events
    OnScroll(InfiniteScrollEvent),
    PosterItem(PosterItem),
    ClosePopup,

    Ignore,
}

pub struct AuthorListPage {
    media_items: Option<Vec<Person>>,
    total_media_count: usize,

    is_fetching_authors: bool,

    media_popup: Option<DisplayOverlay>,

    author_list_ref: NodeRef,
}

impl Component for AuthorListPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            media_items: None,
            total_media_count: 0,
            is_fetching_authors: false,
            media_popup: None,
            author_list_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RequestPeople => {
                if self.is_fetching_authors {
                    return false;
                }

                self.is_fetching_authors = true;

                let offset = Some(
                    self.media_items
                        .as_ref()
                        .map(|v| v.len())
                        .unwrap_or_default(),
                )
                .filter(|v| *v != 0);

                ctx.link().send_future(async move {
                    Msg::PeopleListResults(request::get_people(None, offset, None).await)
                });
            }

            Msg::PeopleListResults(resp) => {
                self.is_fetching_authors = false;

                match resp.ok() {
                    Ok(mut resp) => {
                        self.total_media_count = resp.total;

                        if let Some(items) = self.media_items.as_mut() {
                            items.append(&mut resp.items);
                        } else {
                            self.media_items = Some(resp.items);
                        }
                    }
                    Err(err) => crate::display_error(err),
                }
            }

            Msg::PersonUpdateSearchResults(search_value, resp) => {
                if let Some(DisplayOverlay::SearchForPerson {
                    response,
                    input_value,
                    ..
                }) = self.media_popup.as_mut()
                {
                    match resp.ok() {
                        Ok(resp) => {
                            *response = Some(resp);
                            *input_value = Some(search_value);
                        }
                        Err(err) => crate::display_error(err),
                    }
                }
            }

            Msg::PersonCombineSearchResults(search_value, resp) => {
                if let Some(DisplayOverlay::CombinePersonWith {
                    response,
                    input_value,
                    ..
                }) = self.media_popup.as_mut()
                {
                    match resp.ok() {
                        Ok(resp) => {
                            *response = Some(resp);
                            *input_value = Some(search_value);
                        }
                        Err(err) => crate::display_error(err),
                    }
                }
            }

            Msg::OnScroll(event) => {
                if event.scroll_height - event.scroll_pos < 600 && self.can_req_more() {
                    ctx.link().send_message(Msg::RequestPeople);
                }

                return false;
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

                PosterItem::UpdatePerson(person_id) => {
                    ctx.link().send_future(async move {
                        request::update_person(person_id, &api::PostPersonBody::AutoMatchById)
                            .await;

                        Msg::Ignore
                    });
                }
            },

            Msg::ClosePopup => {
                self.media_popup = None;
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
        if first_render {
            ctx.link().send_message(Msg::RequestPeople);
        }
    }
}

impl AuthorListPage {
    fn render_main(&self, ctx: &Context<Self>) -> Html {
        if let Some(items) = self.media_items.as_deref() {
            // TODO: Placeholders
            // let remaining = (self.total_media_count as usize - items.len()).min(50);

            html! {
                <InfiniteScroll
                    ref={ self.author_list_ref.clone() }
                    class="person-list"
                    event={ ctx.link().callback(Msg::OnScroll) }
                >
                    { for items.iter().map(|item| self.render_media_item(item, ctx.link())) }
                    // { for (0..remaining).map(|_| Self::render_placeholder_item()) }

                    {
                        if let Some(overlay_type) = self.media_popup.as_ref() {
                            match overlay_type {
                                DisplayOverlay::Info { person_id: _ } => {
                                    html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <h1>{"Info"}</h1>
                                        </Popup>
                                    }
                                }

                                &DisplayOverlay::More { person_id, mouse_pos } => {
                                    html! {
                                        <Popup type_of={ PopupType::AtPoint(mouse_pos.0, mouse_pos.1) } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <div class="menu-list">
                                                <PopupClose class="menu-item">{ "Start Reading" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_scope(ctx.link().clone(), move |_| Msg::PosterItem(PosterItem::UpdatePerson(person_id)))
                                                }>{ "Refresh Person" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop_scope(ctx.link().clone(), move |_| Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::SearchForPerson { person_id, input_value: None, response: None })))
                                                }>{ "Search For Person" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop_scope(ctx.link().clone(), move |_| Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::CombinePersonWith { person_id, input_value: None, response: None })))
                                                } title="Join Person into Another">{ "Join Into Person" }</PopupClose>
                                                <PopupClose class="menu-item">{ "Delete" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop_scope(ctx.link().clone(), move |_| Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::Info { person_id })))
                                                }>{ "Show Info" }</PopupClose>
                                            </div>
                                        </Popup>
                                    }
                                }

                                &DisplayOverlay::SearchForPerson { person_id, ref input_value, ref response } => {
                                    let input_id = "external-person-search-input";

                                    let input_value = if let Some(v) = input_value {
                                        v.to_string()
                                    } else {
                                        let items = self.media_items.as_ref().unwrap();
                                        items.iter().find(|v| v.id == person_id).unwrap().name.clone()
                                    };

                                    html! {
                                        <Popup
                                            type_of={ PopupType::FullOverlay }
                                            on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                            classes={ classes!("external-person-search-popup") }
                                        >
                                            <h1>{"Person Search"}</h1>

                                            <form>
                                                <input id={input_id} name="person_search" placeholder="Search For Person" value={ input_value } />
                                                <button onclick={
                                                    ctx.link().callback_future(move |e: MouseEvent| async move {
                                                        e.prevent_default();

                                                        let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

                                                        Msg::PersonUpdateSearchResults(input.value(), request::search_for(&input.value(), SearchType::Person).await)
                                                    })
                                                }>{ "Search" }</button>
                                            </form>

                                            <div class="external-person-search-container">
                                                {
                                                    if let Some(resp) = response {
                                                        html! {
                                                            {
                                                                for resp.items.iter()
                                                                    .map(|(site, items)| {
                                                                        html! {
                                                                            <>
                                                                                <h2>{ site.clone() }</h2>
                                                                                <div class="person-search-items">
                                                                                    {
                                                                                        for items.iter()
                                                                                            .map(|item| {
                                                                                                let item = item.as_person();

                                                                                                let source = item.source.clone();

                                                                                                html! { // TODO: Place into own component.
                                                                                                    <PopupClose
                                                                                                        class="person-search-item"
                                                                                                        onclick={
                                                                                                            ctx.link()
                                                                                                            .callback_future(move |_| {
                                                                                                                let source = source.clone();

                                                                                                                async move {
                                                                                                                    request::update_person(person_id, &api::PostPersonBody::UpdateBySource(source)).await;

                                                                                                                    Msg::Ignore
                                                                                                                }
                                                                                                            })
                                                                                                        }
                                                                                                    >
                                                                                                        <img src={ item.cover_image.clone().unwrap_or_default() } />
                                                                                                        <div class="person-info">
                                                                                                            <h4>{ item.name.clone() }</h4>
                                                                                                            <p>{ item.description.clone().map(|mut v| { truncate_on_indices(&mut v, 300); v }).unwrap_or_default() }</p>
                                                                                                        </div>
                                                                                                    </PopupClose>
                                                                                                }
                                                                                            })
                                                                                    }
                                                                                </div>
                                                                            </>
                                                                        }
                                                                    })
                                                            }
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                            </div>
                                        </Popup>
                                    }
                                }

                                &DisplayOverlay::CombinePersonWith { person_id, ref input_value, ref response } => {
                                    let input_id = "external-person-search-input";

                                    let input_value = if let Some(v) = input_value {
                                        v.to_string()
                                    } else {
                                        let items = self.media_items.as_ref().unwrap();
                                        items.iter().find(|v| v.id == person_id).unwrap().name.clone()
                                    };

                                    html! {
                                        <Popup
                                            type_of={ PopupType::FullOverlay }
                                            on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                            classes={ classes!("external-person-search-popup") }
                                        >
                                            <h1>{"Person Search"}</h1>

                                            <form>
                                                <input id={input_id} name="person_search" placeholder="Search For Person" value={ input_value } />
                                                <button onclick={
                                                    ctx.link().callback_future(move |e: MouseEvent| async move {
                                                        e.prevent_default();

                                                        let input = document().get_element_by_id(input_id).unwrap().unchecked_into::<HtmlInputElement>();

                                                        Msg::PersonCombineSearchResults(input.value(), request::get_people(Some(&input.value()), None, None).await.map(|v| v.items))
                                                    })
                                                }>{ "Search" }</button>
                                            </form>

                                            <div class="external-person-search-container">
                                                {
                                                    if let Some(resp) = response {
                                                        html! {
                                                            <>
                                                                <h2>{ "Media Items" }</h2>
                                                                <div class="person-search-items">
                                                                    {
                                                                        for resp.iter().filter(|p| p.id != person_id).map(|item| {
                                                                            let other_person = item.id;

                                                                            html! { // TODO: Place into own component.
                                                                                <PopupClose
                                                                                    class="person-search-item"
                                                                                    onclick={
                                                                                        ctx.link()
                                                                                        .callback_future(move |_| {
                                                                                            async move {
                                                                                                request::update_person(
                                                                                                    person_id,
                                                                                                    &api::PostPersonBody::CombinePersonWith(other_person)
                                                                                                ).await;

                                                                                                Msg::Ignore
                                                                                            }
                                                                                        })
                                                                                    }
                                                                                >
                                                                                    <img src={ item.get_thumb_url() } />
                                                                                    <div class="person-info">
                                                                                        <h4>{ item.name.clone() }</h4>
                                                                                        <p>{ item.description.clone()
                                                                                                .map(|mut v| { truncate_on_indices(&mut v, 300); v })
                                                                                                .unwrap_or_default() }</p>
                                                                                    </div>
                                                                                </PopupClose>
                                                                            }
                                                                        })
                                                                    }
                                                                </div>
                                                            </>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                            </div>
                                        </Popup>
                                    }
                                }
                            }
                        } else {
                            html! {}
                        }
                    }
                </InfiniteScroll>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }

    // TODO: Move into own struct.
    fn render_media_item(&self, item: &Person, scope: &Scope<Self>) -> Html {
        let person_id = item.id;
        let author_list_ref = self.author_list_ref.clone();
        let on_click_more = scope.callback(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            let scroll = author_list_ref.cast::<HtmlElement>().unwrap().scroll_top();

            Msg::PosterItem(PosterItem::ShowPopup(DisplayOverlay::More {
                person_id,
                mouse_pos: (e.page_x(), e.page_y() + scroll),
            }))
        });

        html! {
            <Link<BaseRoute> to={ BaseRoute::ViewPerson { person_id: item.id } } classes={ classes!("person-container") }>
                <div class="photo">
                    <div class="bottom-right">
                        <span class="material-icons" onclick={on_click_more} title="More Options">{ "more_horiz" }</span>
                    </div>
                    <img src={ item.get_thumb_url() } />
                </div>
                <span class="title">{ item.name.clone() }</span>
            </Link<BaseRoute>>
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
        let count = self
            .media_items
            .as_ref()
            .map(|v| v.len())
            .unwrap_or_default();

        count != 0 && count != self.total_media_count as usize
    }
}

#[derive(Clone)]
pub enum PosterItem {
    // Poster Specific Buttons
    ShowPopup(DisplayOverlay),

    // Popup Events
    UpdatePerson(PersonId),
}

#[derive(Clone)]
pub enum DisplayOverlay {
    Info {
        person_id: PersonId,
    },

    More {
        person_id: PersonId,
        mouse_pos: (i32, i32),
    },

    SearchForPerson {
        person_id: PersonId,
        input_value: Option<String>,
        response: Option<api::BookSearchResponse>,
    },

    CombinePersonWith {
        person_id: PersonId,
        input_value: Option<String>,
        response: Option<Vec<Person>>,
    },
}

impl PartialEq for DisplayOverlay {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Info { person_id: l_id }, Self::Info { person_id: r_id }) => l_id == r_id,
            (
                Self::More {
                    person_id: l_id, ..
                },
                Self::More {
                    person_id: r_id, ..
                },
            ) => l_id == r_id,
            (
                Self::SearchForPerson {
                    person_id: l_id,
                    input_value: l_val,
                    ..
                },
                Self::SearchForPerson {
                    person_id: r_id,
                    input_value: r_val,
                    ..
                },
            ) => l_id == r_id && l_val == r_val,

            _ => false,
        }
    }
}
