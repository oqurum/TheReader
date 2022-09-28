// TODO: Handle resizing.
// TODO: Allow custom sizes.

use std::{rc::Rc, sync::{Mutex, Arc}};

use common::{api::WrappingResponse, component::{PopupType, Popup}};
use common_local::{MediaItem, api::{GetChaptersResponse, self}, Progression, FileId};
use gloo_utils::window;
use js_sys::Array;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlInputElement, Element};
use yew::prelude::*;

use crate::{request, components::reader::{LoadedChapters, ChapterDisplay, PageLoadType, PageLoadSettings}};
use crate::components::reader::Reader;
use crate::components::notes::Notes;


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LocalPopupType {
    Notes,
    Settings
}

pub enum Msg {
    // Event
    Update,

    ClosePopup,
    ShowPopup(LocalPopupType),

    OnChangeSelection(ChapterDisplay),
    UpdateDimensions,
    ChangeReaderSize(bool),
    ChangePageLoadType(PageLoadType),

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
    page_load_settings: PageLoadSettings,
    book_display: ChapterDisplay,
    progress: Rc<Mutex<Option<Progression>>>,
    book: Option<Rc<MediaItem>>,
    chapters: Rc<Mutex<LoadedChapters>>,
    last_grabbed_count: usize,
    // TODO: Cache pages

    book_dimensions: (Option<i32>, Option<i32>),
    is_fullscreen: bool,
    auto_resize_cb: Option<Closure<dyn FnMut()>>,

    sidebar_visible: Option<LocalPopupType>,

    // Refs
    ref_width_input: NodeRef,
    ref_height_input: NodeRef,
    ref_book_container: NodeRef,
}

impl Component for ReadingBook {
    type Message = Msg;
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            page_load_settings: PageLoadSettings {
                speed: 1000,
                type_of: PageLoadType::Select,
            },
            book_display: ChapterDisplay::Double,
            chapters: Rc::new(Mutex::new(LoadedChapters::new())),
            last_grabbed_count: 0,
            progress: Rc::new(Mutex::new(None)),
            book: None,

            book_dimensions: (Some(1040), Some(548)),
            is_fullscreen: false,
            auto_resize_cb: None,

            sidebar_visible: None,

            ref_width_input: NodeRef::default(),
            ref_height_input: NodeRef::default(),
            ref_book_container: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Update => (),

            Msg::ChangePageLoadType(type_of) => {
                // TODO: This isn't needed. We'll have to cache it in our db instead.
                self.page_load_settings.type_of = type_of;
            }

            Msg::ChangeReaderSize(value) => {
                self.is_fullscreen = value;

                if value {
                    self.book_dimensions = (None, None);

                    let link = ctx.link().clone();
                    let timeout: Arc<Mutex<(Option<_>, i32)>> = Arc::new(Mutex::new((None, 0)));

                    let handle_resize = Closure::wrap(Box::new(move || {
                        let link = link.clone();
                        let timeoout = timeout.clone();

                        let clear = {
                            let mut lock = timeoout.lock().unwrap();
                            lock.0.take().map(|v| (v, lock.1))
                        };

                        if let Some((_, v)) = clear {
                            window().clear_timeout_with_handle(v);
                        }

                        let handle_timeout = Closure::wrap(Box::new(move || {
                            link.send_message(Msg::Update);
                        }) as Box<dyn FnMut()>);

                        let to = window().set_timeout_with_callback_and_timeout_and_arguments(
                            handle_timeout.as_ref().unchecked_ref(),
                            250,
                            &Array::default()
                        ).unwrap();

                        *timeoout.lock().unwrap() = (Some(handle_timeout), to);
                    }) as Box<dyn FnMut()>);

                    window().add_event_listener_with_callback(
                        "resize",
                        handle_resize.as_ref().unchecked_ref()
                    ).unwrap();

                    self.auto_resize_cb = Some(handle_resize);
                } else {
                    self.book_dimensions = (
                        Some(self.book_dimensions.0.unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_width().max(0)) / 2),
                        Some(self.book_dimensions.1.unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_height().max(0)) / 2),
                    );
                }
            }

            Msg::UpdateDimensions => {
                let width = self.ref_width_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;
                let height = self.ref_height_input.cast::<HtmlInputElement>().unwrap().value_as_number() as i32;

                self.book_dimensions = (Some(width).filter(|v| *v > 0), Some(height).filter(|v| *v > 0));
            }

            Msg::OnChangeSelection(change) => {
                self.book_display = change;
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

            if self.book_dimensions.0.is_none() {
                book_class += " overlay-x";
            }

            if self.book_dimensions.1.is_none() {
                book_class += " overlay-y";
            }

            let (width, height) = (
                self.book_dimensions.0.unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_width().max(0)),
                self.book_dimensions.1.unwrap_or_else(|| self.ref_book_container.cast::<Element>().unwrap().client_height().max(0)),
            );

            // TODO: Loading screen until all chapters have done initial generation.

            let is_fullscreen = self.is_fullscreen;

            html! {
                <div class="reading-container">
                    <div class={book_class} ref={self.ref_book_container.clone()}>
                        {
                            if let Some(visible) = self.sidebar_visible {
                                match visible {
                                    LocalPopupType::Notes => html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::ClosePopup) }>
                                            <Notes book={ Rc::clone(book) } />
                                        </Popup>
                                    },

                                    LocalPopupType::Settings => html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ ctx.link().callback(|_| Msg::ClosePopup) }>
                                            <div class="settings">
                                                <div class="form-container shrink-width-to-content">
                                                    <label for="page-load-select">{ "Page Load Type" }</label>

                                                    <select id="page-load-select">
                                                        <option
                                                            selected={ self.page_load_settings.type_of == PageLoadType::All }
                                                            onclick={ ctx.link().callback(|_| Msg::ChangePageLoadType(PageLoadType::All)) }
                                                        >{ "Load All" }</option>
                                                        <option
                                                            selected={ self.page_load_settings.type_of == PageLoadType::Select }
                                                            onclick={ ctx.link().callback(|_| Msg::ChangePageLoadType(PageLoadType::Select)) }
                                                        >{ "Load When Needed" }</option>
                                                    </select>
                                                </div>

                                                <div class="form-container shrink-width-to-content">
                                                    <label for="screen-size-select">{ "Screen Size Selection" }</label>

                                                    <select id="screen-size-select">
                                                        <option selected={ !is_fullscreen } onclick={ ctx.link().callback(|_| Msg::ChangeReaderSize(false)) }>{ "Specified" }</option>
                                                        <option selected={ is_fullscreen } onclick={ ctx.link().callback(|_| Msg::ChangeReaderSize(true)) }>{ "Full screen" }</option>
                                                    </select>
                                                </div>

                                                {
                                                    if is_fullscreen {
                                                        html! {}
                                                    } else {
                                                        html! {
                                                            <div class="form-container shrink-width-to-content">
                                                                <label>{ "Screen Width and Height" }</label>

                                                                <div>
                                                                    <input style="width: 100px;" value={ width.to_string() } ref={ self.ref_width_input.clone() } type="number" />
                                                                    <span>{ "x" }</span>
                                                                    <input style="width: 100px;" value={ height.to_string() } ref={ self.ref_height_input.clone() } type="number" />
                                                                </div>

                                                                <button onclick={ ctx.link().callback(|_| Msg::UpdateDimensions) }>{ "Update Dimensions" }</button>
                                                            </div>
                                                        }
                                                    }
                                                }

                                                <div class="form-container shrink-width-to-content">
                                                    <label for="page-type-select">{ "Screen Size Selection" }</label>
                                                    // TODO: Specify based on book type. Epub/Mobi (Single, Double) - PDF (Scroll)
                                                    <select id="page-type-select" onchange={
                                                        ctx.link()
                                                        .callback(|e: Event| Msg::OnChangeSelection(
                                                            e.target().unwrap()
                                                                .unchecked_into::<web_sys::HtmlSelectElement>()
                                                                .value()
                                                                .parse::<u8>().unwrap()
                                                                .into()
                                                        ))
                                                    }>
                                                        <option value="0" selected={ self.book_display == ChapterDisplay::Single }>{ "Single Page" }</option>
                                                        <option value="1" selected={ self.book_display == ChapterDisplay::Double }>{ "Double Page" }</option>
                                                        <option value="2" selected={ self.book_display == ChapterDisplay::Scroll }>{ "Scrolling Page" }</option>
                                                    </select>
                                                </div>
                                            </div>
                                        </Popup>
                                    },
                                }
                            } else {
                                html! {}
                            }
                        }

                        <div class="tools">
                            <button class="tool-item" title="Open/Close the Notebook" onclick={ ctx.link().callback(|_| Msg::ShowPopup(LocalPopupType::Notes)) }>{ "📝" }</button>
                            <button class="tool-item" title="Open/Close the Settings" onclick={ ctx.link().callback(|_| Msg::ShowPopup(LocalPopupType::Settings)) }>{ "⚙️" }</button>
                        </div>

                        <Reader
                            settings={ self.page_load_settings.clone() }
                            display={ self.book_display }
                            progress={ Rc::clone(&self.progress) }
                            book={ Rc::clone(book) }
                            chapters={ Rc::clone(&self.chapters) }
                            dimensions={ (width, height) }
                            request_chapters={ ctx.link().callback(|_| Msg::SendGetChapters) }
                        />
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
            let id = ctx.props().id;

            ctx.link().send_future(async move {
                Msg::RetrieveBook(request::get_book_info(id).await)
            });
        }
    }
}

impl ReadingBook {
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

        match self.page_load_settings.type_of {
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