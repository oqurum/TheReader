use common::{component::{multi_select::{MultiSelectItem, MultiSelectModule, MultiSelectEvent}, popup::{Popup, PopupType}}, PersonId, Either, api::WrappingResponse};
use common_local::{api::{GetBookResponse, GetPostersResponse, ApiGetPeopleResponse, PostBookBody}, Person, BookEdit};
use gloo_timers::callback::Timeout;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::request;


#[derive(Clone, Copy)]
pub enum TabDisplay {
    General,
    Poster,
    Info,
}


#[derive(Properties, PartialEq)]
pub struct Property {
    #[prop_or_default]
    pub classes: Classes,

    pub on_close: Callback<()>,

    pub media_resp: GetBookResponse,
}


pub enum Msg {
    RetrievePostersResponse(WrappingResponse<GetPostersResponse>),
    RetrievePeopleResponse(WrappingResponse<ApiGetPeopleResponse>),

    // Events
    SwitchTab(TabDisplay),

    UpdatedPoster,

    SearchPerson(String),
    TogglePerson { toggle: bool, id: PersonId },

    Edit(Box<dyn Fn(&mut BookEdit, String, &GetBookResponse)>, String),
    Save,

    Ignore,
}


pub struct PopupEditBook {
    tab_display: TabDisplay,

    cached_posters: Option<GetPostersResponse>,

    search_timeout: Option<Timeout>,

    selected_persons: Vec<Person>,
    person_search_cache: Vec<Person>,

    edits: BookEdit,
}

impl Component for PopupEditBook {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            tab_display: TabDisplay::General,
            cached_posters: None,
            search_timeout: None,

            selected_persons: ctx.props().media_resp.people.clone(),
            person_search_cache: Vec::new(),

            edits: BookEdit::default(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        self.selected_persons = ctx.props().media_resp.people.clone();
        self.edits = BookEdit::default();

        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => {
                return false;
            }

            Msg::Edit(func, value) => {
                func(&mut self.edits, value, &ctx.props().media_resp);
            }

            Msg::SwitchTab(value) => {
                self.tab_display = value;
                self.cached_posters = None;
            }

            Msg::RetrievePostersResponse(resp) => {
                match resp.ok() {
                    Ok(resp) => self.cached_posters = Some(resp),
                    Err(err) => crate::display_error(err),
                }
            }

            Msg::RetrievePeopleResponse(resp) => {
                match resp.ok() {
                    Ok(resp) => {
                        self.person_search_cache = resp.items;

                        self.person_search_cache.sort_unstable_by(|a, b| a.name.cmp(&b.name));
                    }

                    Err(err) => crate::display_error(err),
                }
            }

            Msg::UpdatedPoster => {
                let meta_id = ctx.props().media_resp.book.id;

                ctx.link()
                .send_future(async move {
                    Msg::RetrievePostersResponse(request::get_posters_for_book(meta_id).await)
                });

                return false;
            }

            Msg::SearchPerson(text) => {
                let scope = ctx.link().clone();
                self.search_timeout = Some(Timeout::new(250, move || {
                    scope.send_future(async move {
                        Msg::RetrievePeopleResponse(request::get_people(Some(&text), None, None).await)
                    });
                }));

                return false;
            }

            Msg::TogglePerson { toggle, id } => {
                // Is person currently set for book
                let is_current = ctx.props().media_resp.people.iter().any(|v| v.id == id);

                match (is_current, toggle) {
                    // Stored and added (again) - not possible
                    (true, true) => self.edits.remove_person(id),
                    // Stored and removed
                    (true, false) => self.edits.insert_removed_person(id),
                    // Not stored and added
                    (false, true) => self.edits.insert_added_person(id),
                    // Not stored and removed
                    (false, false) => self.edits.remove_person(id),
                }

                // TODO: Remove self.selected_persons, utilize self.edits instead
                if toggle {
                    if let Some(person) = self.person_search_cache.iter().find(|v| v.id == id) {
                        self.selected_persons.push(person.clone());
                    }
                } else if let Some(index) = self.selected_persons.iter().position(|v| v.id == id) {
                    self.selected_persons.remove(index);
                }
            }

            Msg::Save => {
                let edit = self.edits.clone();
                let id = ctx.props().media_resp.book.id;
                let close = ctx.props().on_close.clone();

                ctx.link()
                .send_future(async move {
                    let resp = request::update_book(id, &PostBookBody::Edit(edit)).await;

                    if let Err(err) = resp.ok() {
                        crate::display_error(err);
                    }

                    close.emit(());

                    Msg::Ignore
                });
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.props().on_close.clone();

        html! {
            <Popup
                type_of={ PopupType::FullOverlay }
                on_close={ on_close.clone() }
                classes={ classes!("popup-book-edit") }
            >
                <div class="header">
                    <h2>{"Edit"}</h2>
                </div>

                <div class="tab-bar">
                    <div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::General))}>{ "General" }</div>
                    <div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::Poster))}>{ "Poster" }</div>
                    <div class="tab-bar-item" onclick={ctx.link().callback(|_| Msg::SwitchTab(TabDisplay::Info))}>{ "Info" }</div>
                </div>

                { self.render_tab_contents(ctx) }

                <div class="footer">
                    <button class="red" onclick={ Callback::from(move |_| on_close.emit(())) }>{ "Cancel" }</button>
                    <button class="green" onclick={ ctx.link().callback(|_| Msg::Save) }>{ "Save" }</button>
                </div>
            </Popup>
        }
    }
}

impl PopupEditBook {
    fn render_tab_contents(&self, ctx: &Context<Self>) -> Html {
        match self.tab_display {
            TabDisplay::General => self.render_tab_general(ctx),
            TabDisplay::Poster => {
                if self.cached_posters.is_none() {
                    let metadata_id = ctx.props().media_resp.book.id;

                    ctx.link()
                    .send_future(async move {
                        Msg::RetrievePostersResponse(request::get_posters_for_book(metadata_id).await)
                    });
                }

                self.render_tab_poster(ctx)
            },
            TabDisplay::Info => self.render_tab_info(ctx.props()),
        }
    }


    fn render_tab_general(&self, ctx: &Context<Self>) -> Html {
        let resp = &ctx.props().media_resp;

        html! {
            <div class="content">
                <div class="form-container">
                    <label for="input-title">{ "Title" }</label>
                    <input
                        type="text" id="input-title"
                        value={ self.edits.title.as_ref().map_or_else(|| resp.book.title.clone(), |v| v.clone()) }
                        onchange={
                            ctx.link().callback(move |e: Event| Msg::Edit(
                                Box::new(|e, v, c| { e.title = Some(Some(v).filter(|v| !v.trim().is_empty())).filter(|v| v != &c.book.title); }),
                                e.target_unchecked_into::<HtmlInputElement>().value(),
                            ))
                        }
                    />
                </div>

                <div class="form-container">
                    <label for="input-orig-title">{ "Original Title" }</label>
                    <input
                        type="text" id="input-orig-title"
                        value={ self.edits.original_title.as_ref().map_or_else(|| resp.book.original_title.clone(), |v| v.clone()) }
                        onchange={
                            ctx.link().callback(move |e: Event| Msg::Edit(
                                Box::new(|e, v, c| { e.original_title = Some(Some(v).filter(|v| !v.trim().is_empty())).filter(|v| v != &c.book.original_title); }),
                                e.target_unchecked_into::<HtmlInputElement>().value(),
                            ))
                        }
                    />
                </div>

                <div class="form-container">
                    <label for="input-descr">{ "Description" }</label>
                    <textarea
                        type="text" id="input-descr" rows="5"
                        value={ self.edits.description.as_ref().map_or_else(|| resp.book.description.clone(), |v| v.clone()) }
                        onchange={
                            ctx.link().callback(move |e: Event| Msg::Edit(
                                Box::new(|e, v, c| { e.description = Some(Some(v).filter(|v| !v.trim().is_empty())).filter(|v| v != &c.book.description); }),
                                e.target_unchecked_into::<HtmlTextAreaElement>().value(),
                            ))
                        }
                    />
                </div>

                <div class="form-container">
                    <span>{ "People" }</span>

                    <MultiSelectModule<PersonId>
                        editing=true
                        create_new=false
                        on_event={
                            ctx.link().callback(|v| match v {
                                MultiSelectEvent::Toggle { toggle, id } => {
                                    Msg::TogglePerson { toggle, id }
                                }

                                MultiSelectEvent::Input { text } => {
                                    Msg::SearchPerson(text)
                                }

                                MultiSelectEvent::Create(_) => Msg::Ignore,
                            })
                        }
                    >
                        {
                            for self.selected_persons.iter()
                                .map(|person| html_nested! {
                                    <MultiSelectItem<PersonId> id={ person.id } name={ person.name.clone() } selected=true />
                                })
                        }
                        {
                            for self.person_search_cache.iter()
                                .filter(|v| !self.selected_persons.iter().any(|z| v.id == z.id))
                                .map(|person| html_nested! {
                                    <MultiSelectItem<PersonId> id={ person.id } name={ person.name.clone() } />
                                })
                        }
                    </MultiSelectModule<PersonId>>
                </div>
            </div>
        }
    }

    fn render_tab_poster(&self, ctx: &Context<Self>) -> Html {
        if let Some(resp) = self.cached_posters.as_ref() {
            html! {
                <div class="content edit-posters">
                    <div class="drop-container">
                        <h4>{ "Drop File To Upload" }</h4>
                    </div>
                    <div class="poster-list">
                        {
                            for resp.items.iter().map(|poster| {
                                let meta_id = ctx.props().media_resp.book.id;
                                let url_or_id = poster.id.map(Either::Right).unwrap_or_else(|| Either::Left(poster.path.clone()));
                                let is_selected = poster.selected;

                                html_nested! {
                                    <div
                                        class={ classes!("poster", { if is_selected { "selected" } else { "" } }) }
                                        onclick={ctx.link().callback_future(move |_| {
                                            let url_or_id = url_or_id.clone();

                                            async move {
                                                if is_selected {
                                                    Msg::Ignore
                                                } else {
                                                    request::change_poster_for_book(meta_id, url_or_id).await;

                                                    Msg::UpdatedPoster
                                                }
                                            }
                                        })}
                                    >
                                        <img src={poster.path.clone()} />
                                    </div>
                                }
                            })
                        }
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="content edit-posters">
                    <h3>{ "Loading Posters..." }</h3>
                </div>
            }
        }
    }

    fn render_tab_info(&self, _props: &<Self as Component>::Properties) -> Html {
        html! {
            <div class="content">
            </div>
        }
    }
}