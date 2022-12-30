use std::{cell::RefCell, rc::Rc};

use common::{
    api::WrappingResponse,
    component::{
        multi_select::{MultiSelectEvent, MultiSelectItem, MultiSelectModule},
        popup::{button::{ButtonWithPopup, ButtonPopupPosition}, Popup, PopupClose, PopupType},
    },
    BookId, PersonId,
};
use common_local::{
    api::{self, ApiGetPeopleResponse, MassEditBooks},
    ModifyValuesBy, Person,
};
use gloo_timers::callback::Timeout;
use web_sys::{HtmlElement, HtmlSelectElement};
use yew::prelude::*;

use crate::request;

static EDITING_CONTAINER_CLASS: &str = "editing-items-inside";

#[derive(Properties)]
pub struct Property {
    pub on_deselect_all: Callback<MouseEvent>,

    pub editing_container: NodeRef,

    pub editing_items: Rc<RefCell<Vec<BookId>>>,
}

impl PartialEq for Property {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

pub enum Msg {
    SaveResponse(WrappingResponse<String>),

    Ignore,
    // TogglePopup,
    UpdateMultiple(api::PostBookBody),

    EditPopupMsg(MsgEditPopup),
    ShowEditPopup(LocalPopupType),
    CloseEditPopup,
}

pub enum MsgEditPopup {
    SearchText(String),
    TogglePerson { toggle: bool, id: PersonId },
    PeopleResponse(WrappingResponse<ApiGetPeopleResponse>),

    UpdateEdit(Box<dyn Fn(&mut MassEditBooks, String)>, String),
    Save,
}

pub struct MassSelectBar {
    popup_display: Option<LocalPopupType>,
    search_timeout: Option<Timeout>,
}

impl Component for MassSelectBar {
    type Message = Msg;
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            popup_display: None,
            search_timeout: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SaveResponse(resp) => {
                if let Err(err) = resp.ok() {
                    crate::display_error(err);
                }
            }

            Msg::UpdateMultiple(type_of) => {
                let book_ids = ctx.props().editing_items.borrow().clone();

                ctx.link().send_future(async move {
                    for book_id in book_ids {
                        request::update_book(book_id, &type_of).await;
                    }

                    Msg::Ignore
                });
            }

            Msg::ShowEditPopup(v) => {
                self.popup_display = Some(v);
            }

            Msg::CloseEditPopup => {
                self.popup_display = None;
            }

            Msg::EditPopupMsg(msg) => {
                if let Some(popup) = self.popup_display.as_mut() {
                    match popup {
                        LocalPopupType::EditBooks {
                            selected_people,
                            cached_people,
                            edit,
                        } => match msg {
                            MsgEditPopup::SearchText(search) => {
                                let scope = ctx.link().clone();
                                self.search_timeout = Some(Timeout::new(250, move || {
                                    scope.send_future(async move {
                                        Msg::EditPopupMsg(MsgEditPopup::PeopleResponse(
                                            request::get_people(Some(&search), None, None).await,
                                        ))
                                    });
                                }));

                                return false;
                            }

                            MsgEditPopup::TogglePerson { toggle, id } => {
                                if toggle {
                                    if let Some(person) = cached_people.iter().find(|v| v.id == id)
                                    {
                                        selected_people.push(person.clone());
                                        edit.people_list.push(person.id);
                                    }
                                } else {
                                    if let Some(index) =
                                        selected_people.iter().position(|v| v.id == id)
                                    {
                                        selected_people.remove(index);
                                    }

                                    if let Some(index) =
                                        edit.people_list.iter().position(|v| *v == id)
                                    {
                                        edit.people_list.remove(index);
                                    }
                                }
                            }

                            MsgEditPopup::PeopleResponse(resp) => match resp.ok() {
                                Ok(resp) => *cached_people = resp.items,
                                Err(err) => crate::display_error(err),
                            },

                            MsgEditPopup::UpdateEdit(func, input) => {
                                func(edit, input);
                            }

                            MsgEditPopup::Save => {
                                edit.book_ids = ctx.props().editing_items.borrow().clone();

                                let edit = edit.clone();

                                ctx.link().send_future(async move {
                                    Msg::SaveResponse(request::update_books(&edit).await)
                                });
                            }
                        },
                    }
                }
            }

            Msg::Ignore => return false,
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let items = ctx.props().editing_items.borrow();

        if items.is_empty() {
            html! {}
        } else {
            html! {
                <div class="mass-select-bar">
                    <div class="bar-container">
                        <div class="left-content">
                            <span>{ items.len() } { " items selected" }</span>
                        </div>
                        <div class="center-content">
                            <ButtonWithPopup class="menu-list" position={ ButtonPopupPosition::Top }>
                                <PopupClose class="menu-item" onclick={ ctx.link().callback(move |e: MouseEvent| {
                                    e.prevent_default();
                                    Msg::UpdateMultiple(api::PostBookBody::RefreshBookId)
                                }) }>
                                    { "Refresh Metadata" }
                                </PopupClose>
                                <PopupClose class="menu-item" onclick={ ctx.link().callback(move |e: MouseEvent| {
                                    e.prevent_default();
                                    Msg::UpdateMultiple(api::PostBookBody::AutoMatchBookIdByFiles)
                                }) }>
                                    { "Quick Search By Files" }
                                </PopupClose>
                                <PopupClose class="menu-item">{ "Delete" }</PopupClose>
                            </ButtonWithPopup>

                            <button class="btn btn-secondary btn-sm" onclick={ ctx.link().callback(move |e: MouseEvent| {
                                e.prevent_default();
                                Msg::ShowEditPopup(LocalPopupType::default_edit_books())
                            }) }>
                                <span class="material-icons" title="Edit Items">{ "edit" }</span>
                            </button>
                        </div>
                        <div class="right-content">
                            <button class="btn btn-danger btn-sm" onclick={ ctx.props().on_deselect_all.clone() }>{ "Deselect All" }</button>
                        </div>
                    </div>

                    {
                        if let Some(popup) = self.popup_display.as_ref() {
                            match popup {
                                LocalPopupType::EditBooks { selected_people, cached_people, edit } => html! {
                                    <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::CloseEditPopup) } classes={ classes!("popup-book-edit") }>
                                        <div class="modal-body">
                                            // Update People
                                            <div class="mb-3">
                                                <h4>{ "People" }</h4>

                                                <MultiSelectModule<PersonId>
                                                    editing=true
                                                    create_new=false
                                                    on_event={
                                                        ctx.link().callback(|v| match v {
                                                            MultiSelectEvent::Toggle { toggle, id } => Msg::EditPopupMsg(MsgEditPopup::TogglePerson { toggle, id }),
                                                            MultiSelectEvent::Input { text } => Msg::EditPopupMsg(MsgEditPopup::SearchText(text)),
                                                            MultiSelectEvent::Create(_) => Msg::Ignore,
                                                        })
                                                    }
                                                >
                                                    {
                                                        for selected_people.iter()
                                                            .map(|person| html_nested! {
                                                                <MultiSelectItem<PersonId> id={ person.id } name={ person.name.clone() } selected=true />
                                                            })
                                                    }
                                                    {
                                                        for cached_people.iter()
                                                            .filter(|v| !selected_people.iter().any(|z| v.id == z.id))
                                                            .map(|person| html_nested! {
                                                                <MultiSelectItem<PersonId> id={ person.id } name={ person.name.clone() } />
                                                            })
                                                    }
                                                </MultiSelectModule<PersonId>>

                                                <select class="form-select" onchange={ ctx.link().callback(|v: Event| Msg::EditPopupMsg(MsgEditPopup::UpdateEdit(
                                                    Box::new(|e, v| { e.people_list_mod = ModifyValuesBy::from(v.parse::<u8>().unwrap()); }),
                                                    v.target_unchecked_into::<HtmlSelectElement>().selected_index().to_string()
                                                ))) }>
                                                    <option value="0" selected={ edit.people_list_mod as u8 == 0 }>{ "Overwrite" }</option>
                                                    <option value="1" selected={ edit.people_list_mod as u8 == 1 }>{ "Append" }</option>
                                                    <option value="2" selected={ edit.people_list_mod as u8 == 2 }>{ "Remove" }</option>
                                                </select>
                                            </div>
                                        </div>

                                        <div class="modal-footer">
                                            <button class="btn btn-danger" onclick={ ctx.link().callback(|_| Msg::CloseEditPopup) }>{ "Cancel" }</button>
                                            <button class="btn btn-success" onclick={ ctx.link().callback(|_| Msg::EditPopupMsg(MsgEditPopup::Save)) }>{ "Save" }</button>
                                        </div>
                                    </Popup>
                                },
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _prev: &Self::Properties) -> bool {
        if let Some(container_element) = ctx.props().editing_container.cast::<HtmlElement>() {
            let cl = container_element.class_list();

            if ctx.props().editing_items.borrow().is_empty() {
                let _ = cl.remove_1(EDITING_CONTAINER_CLASS);
            } else if !cl.contains(EDITING_CONTAINER_CLASS) {
                let _ = cl.add_1(EDITING_CONTAINER_CLASS);
            }
        }

        true
    }
}

#[derive(Clone, PartialEq)]
pub enum LocalPopupType {
    EditBooks {
        edit: MassEditBooks,
        selected_people: Vec<Person>,
        cached_people: Vec<Person>,
    },
}

impl LocalPopupType {
    pub fn default_edit_books() -> Self {
        Self::EditBooks {
            edit: MassEditBooks::default(),
            selected_people: Default::default(),
            cached_people: Default::default(),
        }
    }
}
