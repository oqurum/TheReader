// TODO: Handle resizing.

use std::{rc::Rc, sync::{Mutex, Arc}};

use common::{api::WrappingResponse, component::{PopupType, Popup}};
use common_local::{MediaItem, api::{GetChaptersResponse, self}, Progression, FileId};
use gloo_timers::callback::Timeout;
use gloo_utils::window;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlInputElement, Element};
use yew::{prelude::*, html::Scope};

use crate::{request, components::reader::{LoadedChapters, ChapterDisplay, PageLoadType, ReaderSettings}};
use crate::components::reader::Reader;
use crate::components::notes::Notes;


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LocalPopupType {
    Notes,
    Settings
}

pub enum Msg {
    // Event
    WindowResize,

    ClosePopup,
    ShowPopup(LocalPopupType),

    ChangeReaderSettings(ReaderSettings),

    // Send
    SendGetChapters,

    // Retrieve
    RetrieveBook(WrappingResponse<api::ApiGetFileByIdResponse>),
    RetrievePages(WrappingResponse<GetChaptersResponse>),
}

#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub id: FileId
}

pub struct ReadingBook {
    reader_settings: ReaderSettings,
    progress: Rc<Mutex<Option<Progression>>>,
    book: Option<Rc<MediaItem>>,
    chapters: Rc<Mutex<LoadedChapters>>,
    last_grabbed_count: usize,
    // TODO: Cache pages

    auto_resize_cb: Option<Closure<dyn FnMut()>>,

    sidebar_visible: Option<LocalPopupType>,

    // Refs
    ref_book_container: NodeRef,
}

impl Component for ReadingBook {
    type Message = Msg;
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            reader_settings: ReaderSettings {
                load_speed: 1000,
                type_of: PageLoadType::Select,
                is_fullscreen: false,
                display: ChapterDisplay::Double,
                dimensions: (1040, 548),
            },
            chapters: Rc::new(Mutex::new(LoadedChapters::new())),
            last_grabbed_count: 0,
            progress: Rc::new(Mutex::new(None)),
            book: None,

            auto_resize_cb: None,

            sidebar_visible: None,

            ref_book_container: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::WindowResize => {
                if self.reader_settings.is_fullscreen {
                    let cont = self.ref_book_container.cast::<Element>().unwrap();
                    self.reader_settings.dimensions = (cont.client_width().max(0), cont.client_height().max(0));
                } else {
                    return false;
                }
            }

            Msg::ChangeReaderSettings(new_settings) => {
                // Replace old settings with new settings.
                let old_settings = std::mem::replace(&mut self.reader_settings, new_settings);

                if self.reader_settings.is_fullscreen && old_settings.is_fullscreen != self.reader_settings.is_fullscreen {
                    let cont = self.ref_book_container.cast::<Element>().unwrap();

                    // TODO: client_height is incorrect since the tools is set to absolute after this update.
                    self.reader_settings.dimensions = (cont.client_width().max(0), cont.client_height().max(0));
                } else if !old_settings.is_fullscreen {
                    self.reader_settings.dimensions = (
                        Some(self.reader_settings.dimensions.0).filter(|v| *v > 0).unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_width().max(0)) / 2,
                        Some(self.reader_settings.dimensions.1).filter(|v| *v > 0).unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_height().max(0)) / 2,
                    );
                }
            }

            Msg::ClosePopup => {
                self.sidebar_visible = None;
            }

            Msg::ShowPopup(type_of) => {
                match self.sidebar_visible {
                    Some(v) if v == type_of => { self.sidebar_visible = None; },
                    _ => self.sidebar_visible = Some(type_of),
                }
            }

            Msg::RetrievePages(resp) => {
                match resp.ok() {
                    Ok(mut info) => {
                        let mut chap_container = self.chapters.lock().unwrap();

                        self.last_grabbed_count = info.limit;
                        chap_container.total = info.total;

                        chap_container.chapters.append(&mut info.items);
                    },

                    Err(err) => crate::display_error(err),
                }
            }

            Msg::RetrieveBook(resp) => {
                match resp.ok() {
                    Ok(Some(resp)) => {
                        self.book = Some(Rc::new(resp.media));
                        *self.progress.lock().unwrap() = resp.progress;
                        // TODO: Check to see if we have progress. If so, generate those pages first.
                        ctx.link().send_message(Msg::SendGetChapters);
                    },

                    Ok(None) => (),

                    Err(err) => crate::display_error(err),
                }
            }

            Msg::SendGetChapters => {
                let book_id = self.book.as_ref().unwrap().id;

                let (start, end) = self.get_next_pages_to_load();

                if end != 0 {
                    ctx.link()
                    .send_future(async move {
                        Msg::RetrievePages(request::get_book_pages(book_id, start, end).await)
                    });
                }

                return false;
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if let Some(book) = self.book.as_ref() {
            let mut book_class = String::from("book");

            if self.reader_settings.is_fullscreen {
                book_class += " overlay-x overlay-y";
            }

            // TODO: Loading screen until sections have done initial generation.

            html! {
                <div class="reading-container">
                    <div class={ book_class } ref={self.ref_book_container.clone()}>
                        {
                            if let Some(visible) = self.sidebar_visible {
                                match visible {
                                    LocalPopupType::Notes => html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::ClosePopup) }>
                                            <Notes book={ Rc::clone(book) } />
                                        </Popup>
                                    },

                                    LocalPopupType::Settings => html! {
                                        <SettingsContainer
                                            scope={ ctx.link().clone() }
                                            reader_dimensions={ self.reader_settings.dimensions }
                                            reader_settings={ self.reader_settings.clone() }
                                        />
                                    },
                                }
                            } else {
                                html! {}
                            }
                        }

                        <Reader
                            settings={ self.reader_settings.clone() }
                            progress={ Rc::clone(&self.progress) }
                            book={ Rc::clone(book) }
                            chapters={ Rc::clone(&self.chapters) }
                            request_chapters={ ctx.link().callback(|_| Msg::SendGetChapters) }
                        />
                    </div>

                    <div class={ classes!("tools", self.reader_settings.is_fullscreen.then_some("overlay")) }>
                        <button class="tool-item" title="Open/Close the Notebook" onclick={ ctx.link().callback(|_| Msg::ShowPopup(LocalPopupType::Notes)) }>{ "📝" }</button>
                        <button class="tool-item" title="Open/Close the Settings" onclick={ ctx.link().callback(|_| Msg::ShowPopup(LocalPopupType::Settings)) }>{ "⚙️" }</button>
                    </div>
                </div>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.init_resize_cb(ctx);

            let id = ctx.props().id;

            ctx.link().send_future(async move {
                Msg::RetrieveBook(request::get_book_info(id).await)
            });
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        if let Some(cb) = self.auto_resize_cb.take() {
            window().remove_event_listener_with_callback(
                "resize",
                cb.as_ref().unchecked_ref()
            ).unwrap();
        }
    }
}

impl ReadingBook {
    fn init_resize_cb(&mut self, ctx: &Context<Self>) {
        let link = ctx.link().clone();
        let timeout: Arc<Mutex<Option<Timeout>>> = Arc::new(Mutex::new(None));

        let handle_resize = Closure::wrap(Box::new(move || {
            let link = link.clone();

            let timeout_cloned = timeout.clone();

            drop(timeout_cloned.lock().unwrap().take());

            let to = Timeout::new(250, move || {
                link.send_message(Msg::WindowResize);
            });

            *timeout_cloned.lock().unwrap() = Some(to);
        }) as Box<dyn FnMut()>);

        window().add_event_listener_with_callback(
            "resize",
            handle_resize.as_ref().unchecked_ref()
        ).unwrap();

        self.auto_resize_cb = Some(handle_resize);
    }

    // TODO: Use Option instead of returning (0, 0)
    fn get_next_pages_to_load(&self) -> (usize, usize) {
        let progress = self.progress.lock().unwrap();
        let chap_cont = self.chapters.lock().unwrap();

        let total_sections = self.book.as_ref().map(|v| v.chapter_count).unwrap_or_default();

        // Starting index
        let curr_section = if let Some(&Progression::Ebook{ chapter, .. }) = progress.as_ref() {
            chapter as usize
        } else {
            0
        };


        let mut chapters = chap_cont.chapters.iter().map(|v| v.value).collect::<Vec<_>>();
        chapters.sort_unstable();

        match self.reader_settings.type_of {
            PageLoadType::All => {
                if chap_cont.chapters.is_empty() {
                    (curr_section.saturating_sub(1), curr_section + 2)
                } else {
                    let mut start_pos = 0;
                    let mut end_pos = 0;

                    for section in chapters {
                        // If end_pos == 0 that means we haven't found a valid section to load.
                        if end_pos == 0 {
                            // We already loaded this section
                            if start_pos == section {
                                start_pos += 1;

                                if start_pos == total_sections {
                                    return (0, 0);
                                }
                            } else {
                                end_pos = start_pos + 1;
                            }
                        } else if end_pos == section || end_pos - start_pos == 4 {
                            break;
                        } else {
                            end_pos += 1;
                        }
                    }

                    // If end_pos is still 0 then we've reached the end of the array.
                    if start_pos != 0 && end_pos == 0 {
                        end_pos = start_pos + 3;
                    }

                    (start_pos, end_pos)
                }
            }

            PageLoadType::Select => {
                if chap_cont.chapters.is_empty() {
                    (curr_section.saturating_sub(1), curr_section + 2)
                } else {
                    let found_previous = curr_section != 0 && chapters.iter().any(|v| *v == curr_section - 1);
                    let found_next = curr_section + 1 != total_sections && chapters.iter().any(|v| *v == curr_section + 1);

                    if !found_previous {
                        (curr_section.saturating_sub(1), curr_section)
                    } else if !found_next {
                        (curr_section + 1, curr_section + 2)
                    } else {
                        (0, 0)
                    }
                }
            }
        }
    }
}



#[derive(Properties)]
struct SettingsContainerProps {
    scope: Scope<ReadingBook>,

    reader_dimensions: (i32, i32),

    reader_settings: ReaderSettings,
}

impl PartialEq for SettingsContainerProps {
    fn eq(&self, other: &Self) -> bool {
        self.reader_dimensions == other.reader_dimensions &&
        self.reader_settings == other.reader_settings
    }
}


#[function_component(SettingsContainer)]
fn _settings_cont(props: &SettingsContainerProps) -> Html {
    let settings = props.reader_settings.clone();
    let settings = use_mut_ref(move || settings);

    let page_load_type_section = {
        let settings_inner = settings.clone();

        html! {
            <div class="form-container shrink-width-to-content">
                <label for="page-load-select">{ "Page Load Type" }</label>

                <select id="page-load-select"
                    onchange={ Callback::from(move |e: Event| {
                        let idx = e.target().unwrap()
                            .unchecked_into::<web_sys::HtmlSelectElement>()
                            .selected_index();

                        match idx {
                            0 => settings_inner.borrow_mut().type_of = PageLoadType::All,
                            1 => settings_inner.borrow_mut().type_of = PageLoadType::Select,

                            _ => ()
                        }
                    })
                }>
                    <option selected={ settings.borrow().type_of == PageLoadType::All }>{ "Load All" }</option>
                    <option selected={ settings.borrow().type_of == PageLoadType::Select }>{ "Load When Needed" }</option>
                </select>
            </div>
        }
    };

    let screen_size_type_section = {
        let settings_inner = settings.clone();

        html! {
            <div class="form-container shrink-width-to-content">
                <label for="screen-size-select">{ "Screen Size Selection" }</label>

                <select id="screen-size-select"
                    onchange={ Callback::from(move |e: Event| {
                        let idx = e.target().unwrap()
                            .unchecked_into::<web_sys::HtmlSelectElement>()
                            .selected_index();

                        settings_inner.borrow_mut().is_fullscreen = idx != 0;
                    })
                }>
                    <option selected={ !settings.borrow().is_fullscreen }>{ "Specified" }</option>
                    <option selected={ settings.borrow().is_fullscreen }>{ "Full screen" }</option>
                </select>
            </div>
        }
    };

    let screen_size_section = {
        if settings.borrow().is_fullscreen {
            html! {}
        } else {
            let settings = settings.clone();

            let ref_width_input = use_node_ref();
            let ref_height_input = use_node_ref();

            html! {
                <div class="form-container shrink-width-to-content">
                    <label>{ "Screen Width and Height" }</label>

                    <div>
                        <input
                            style="width: 100px;"
                            value={ props.reader_dimensions.0.to_string() }
                            ref={ ref_width_input.clone() }
                            type="number"
                        />

                        <span>{ "x" }</span>

                        <input
                            style="width: 100px;"
                            value={ props.reader_dimensions.1.to_string() }
                            ref={ ref_height_input.clone() }
                            type="number"
                        />
                    </div>

                    <button onclick={ Callback::from(move |_| {
                        let width = ref_width_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;
                        let height = ref_height_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;

                        settings.borrow_mut().dimensions = (width, height);
                    }) }>{ "Update Dimensions" }</button>
                </div>
            }
        }
    };

    let reader_view_type_section = {
        let settings_inner = settings.clone();

        html! {
            <div class="form-container shrink-width-to-content">
                <label for="page-type-select">{ "Reader View Type" }</label>
                <select id="page-type-select" onchange={
                    Callback::from(move |e: Event| {
                        let display = e.target().unwrap()
                            .unchecked_into::<web_sys::HtmlSelectElement>()
                            .value()
                            .parse::<u8>().unwrap()
                            .into();

                        settings_inner.borrow_mut().display = display;
                    })
                }>
                    <option value="0" selected={ settings.borrow().display == ChapterDisplay::Single }>{ "Single Page" }</option>
                    <option value="1" selected={ settings.borrow().display == ChapterDisplay::Double }>{ "Double Page" }</option>
                    <option value="2" selected={ settings.borrow().display == ChapterDisplay::Scroll }>{ "Scrolling Page" }</option>
                </select>
            </div>
        }
    };

    html! {
        <Popup type_of={ PopupType::FullOverlay } on_close={ props.scope.callback(|_| Msg::ClosePopup) }>
            <div class="settings">
                { page_load_type_section }

                { screen_size_type_section }

                { screen_size_section }

                { reader_view_type_section }

                <hr />

                <div>
                    <button
                        class="green"
                        onclick={ props.scope.callback(move |_| Msg::ChangeReaderSettings(settings.take())) }
                    >{ "Submit" }</button>
                </div>
            </div>
        </Popup>
    }
}