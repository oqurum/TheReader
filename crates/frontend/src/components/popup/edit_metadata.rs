use common::{component::{multi_select::{MultiSelectItem, MultiSelectModule, MultiSelectEvent}, popup::{Popup, PopupType}}, PersonId, Either};
use common_local::{api::{GetBookResponse, GetPostersResponse, ApiGetPeopleResponse}, Person};
use gloo_timers::callback::Timeout;
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
    RetrievePostersResponse(GetPostersResponse),
    RetrievePeopleResponse(ApiGetPeopleResponse),

    // Events
    SwitchTab(TabDisplay),

    UpdatedPoster,

    SearchPerson(String),
    TogglePerson { toggle: bool, id: PersonId },

    Ignore,
}


pub struct PopupEditMetadata {
    tab_display: TabDisplay,

    cached_posters: Option<GetPostersResponse>,

    search_timeout: Option<Timeout>,

    selected_persons: Vec<Person>,
    person_search_cache: Vec<Person>,
}

impl Component for PopupEditMetadata {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            tab_display: TabDisplay::General,
            cached_posters: None,
            search_timeout: None,

            selected_persons: ctx.props().media_resp.people.clone(),
            person_search_cache: Vec::new(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        self.selected_persons = ctx.props().media_resp.people.clone();

        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => {
                return false;
            }

            Msg::SwitchTab(value) => {
                self.tab_display = value;
                self.cached_posters = None;
            }

            Msg::RetrievePostersResponse(resp) => {
                self.cached_posters = Some(resp);
            }

            Msg::RetrievePeopleResponse(resp) => {
                self.person_search_cache = resp.items;
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
                }))
            }

            Msg::TogglePerson { toggle, id } => {
                let book_id = ctx.props().media_resp.book.id;

                if toggle {
                    if let Some(person) = self.person_search_cache.iter().find(|v| v.id == id) {
                        self.selected_persons.push(person.clone());
                    }
                } else if let Some(index) = self.selected_persons.iter().position(|v| v.id == id) {
                    self.selected_persons.remove(index);
                }

                ctx.link()
                .send_future(async move {
                    // TODO: Check Response
                    if toggle {
                        request::add_person_to_book(book_id, id).await;
                    } else {
                        request::delete_person_from_book(book_id, id).await;
                    }

                    Msg::Ignore
                });
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <Popup
                type_of={ PopupType::FullOverlay }
                on_close={ ctx.props().on_close.clone() }
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
                    <button class="red">{ "Cancel" }</button>
                    <button class="green">{ "Save" }</button>
                </div>
            </Popup>
        }
    }
}

impl PopupEditMetadata {
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
                    <input type="text" id="input-title" value={ resp.book.title.clone().unwrap_or_default() } />
                </div>

                <div class="form-container">
                    <label for="input-orig-title">{ "Original Title" }</label>
                    <input type="text" id="input-orig-title" value={ resp.book.original_title.clone().unwrap_or_default() } />
                </div>

                <div class="form-container">
                    <label for="input-descr">{ "Description" }</label>
                    <textarea type="text" id="input-descr" rows="5" value={ resp.book.description.clone().unwrap_or_default() } />
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