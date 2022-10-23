// TODO: Handle resizing.

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use common::{
    api::WrappingResponse,
    component::{Popup, PopupType},
};
use common_local::{
    api::{self, GetChaptersResponse},
    FileId, MediaItem, Progression,
};
use gloo_timers::callback::Timeout;
use gloo_utils::window;
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::Element;
use yew::{context::ContextHandle, prelude::*};

use crate::components::reader::Reader;
use crate::components::{notes::Notes, reader::OverlayEvent};
use crate::{
    components::reader::{
        DragType, LoadedChapters, PageLoadType, ReaderEvent, ReaderSettings, SectionDisplay,
    },
    request, AppState,
};

mod settings;

use settings::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LocalPopupType {
    Notes,
    Settings,
}

#[derive(Debug, Clone, Copy)]
enum DisplayToolBars {
    Hidden,
    Expanded,
}

impl DisplayToolBars {
    pub fn is_expanded(self) -> bool {
        matches!(self, Self::Expanded)
    }
}

pub enum Msg {
    // Event
    WindowResize,

    ClosePopup,
    ShowPopup(LocalPopupType),

    ChangeReaderSettings(ReaderSettings),

    // Send
    ReaderEvent(ReaderEvent),

    // Retrieve
    RetrieveBook(WrappingResponse<api::ApiGetFileByIdResponse>),
    RetrievePages(WrappingResponse<GetChaptersResponse>),

    ContextChanged(Rc<AppState>),
}

#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub id: FileId,
}

pub struct ReadingBook {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    reader_settings: ReaderSettings,
    progress: Rc<Mutex<Option<Progression>>>,
    book: Option<Rc<MediaItem>>,
    chapters: Rc<Mutex<LoadedChapters>>,
    last_grabbed_count: usize,
    // TODO: Cache pages
    auto_resize_cb: Option<Closure<dyn FnMut()>>,

    sidebar_visible: Option<LocalPopupType>,

    display_toolbar: DisplayToolBars,
    timeout: Option<Timeout>,

    // Refs
    ref_book_container: NodeRef,
}

impl Component for ReadingBook {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");

        let (win_width, win_height) = (
            window().inner_width().unwrap_throw().as_f64().unwrap(),
            window().inner_height().unwrap_throw().as_f64().unwrap(),
        );

        let (is_fullscreen, dimensions, display_toolbar) =
            if win_width < 1100.0 || win_height < 720.0 {
                state.update_nav_visibility.emit(false);
                (true, (0, 0), DisplayToolBars::Hidden)
            } else {
                (false, DEFAULT_DIMENSIONS, DisplayToolBars::Expanded)
            };

        Self {
            state,
            _listener,

            reader_settings: ReaderSettings {
                load_speed: 1000,
                type_of: PageLoadType::Select,
                is_fullscreen,
                display: SectionDisplay::new_double(),
                show_progress: false,
                dimensions,
            },
            chapters: Rc::new(Mutex::new(LoadedChapters::new())),
            last_grabbed_count: 0,
            progress: Rc::new(Mutex::new(None)),
            book: None,

            auto_resize_cb: None,

            sidebar_visible: None,
            display_toolbar,
            timeout: None,

            ref_book_container: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => {
                self.state = state;

                // This can be called before everything is rendered.
                if self.ref_book_container.get().is_some() {
                    let link = ctx.link().clone();
                    self.timeout = Some(Timeout::new(100, move || {
                        link.send_message(Msg::WindowResize);
                    }));
                }
            }

            Msg::WindowResize => {
                // We have the display_toolbar check here since the "zoom out" function will re-expand it.
                // We want to ensure we don't "zoom out", resize, and have incorrect dimensions.
                if self.reader_settings.is_fullscreen
                    && !self.display_toolbar.is_expanded()
                    && !self.state.is_navbar_visible
                {
                    let cont = self.ref_book_container.cast::<Element>().unwrap();
                    self.reader_settings.dimensions =
                        (cont.client_width().max(0), cont.client_height().max(0));
                } else {
                    return false;
                }
            }

            Msg::ChangeReaderSettings(new_settings) => {
                // Replace old settings with new settings.
                let old_settings = std::mem::replace(&mut self.reader_settings, new_settings);

                if self.reader_settings.is_fullscreen {
                    let cont = self.ref_book_container.cast::<Element>().unwrap();

                    // TODO: client_height is incorrect since the tools is set to absolute after this update.
                    self.reader_settings.dimensions =
                        (cont.client_width().max(0), cont.client_height().max(0));

                    self.state.update_nav_visibility.emit(false);
                    self.display_toolbar = DisplayToolBars::Hidden;
                } else if !old_settings.is_fullscreen {
                    self.state.update_nav_visibility.emit(true);
                    self.display_toolbar = DisplayToolBars::Expanded;

                    self.reader_settings.dimensions = (
                        Some(self.reader_settings.dimensions.0)
                            .filter(|v| *v > 0)
                            .unwrap_or_else(|| {
                                self.ref_book_container
                                    .cast::<Element>()
                                    .unwrap()
                                    .client_width()
                                    .max(0)
                            })
                            / 2,
                        Some(self.reader_settings.dimensions.1)
                            .filter(|v| *v > 0)
                            .unwrap_or_else(|| {
                                self.ref_book_container
                                    .cast::<Element>()
                                    .unwrap()
                                    .client_height()
                                    .max(0)
                            })
                            / 2,
                    );
                }
            }

            Msg::ClosePopup => {
                self.sidebar_visible = None;
            }

            Msg::ShowPopup(type_of) => match self.sidebar_visible {
                Some(v) if v == type_of => {
                    self.sidebar_visible = None;
                }
                _ => self.sidebar_visible = Some(type_of),
            },

            Msg::RetrievePages(resp) => match resp.ok() {
                Ok(mut info) => {
                    let mut chap_container = self.chapters.lock().unwrap();

                    self.last_grabbed_count = info.limit;
                    chap_container.total = info.total;

                    chap_container.chapters.append(&mut info.items);
                }

                Err(err) => crate::display_error(err),
            },

            Msg::RetrieveBook(resp) => match resp.ok() {
                Ok(Some(resp)) => {
                    self.book = Some(Rc::new(resp.media));
                    *self.progress.lock().unwrap() = resp.progress;
                    ctx.link()
                        .send_message(Msg::ReaderEvent(ReaderEvent::LoadChapters));
                }

                Ok(None) => (),

                Err(err) => crate::display_error(err),
            },

            Msg::ReaderEvent(event) => {
                match event {
                    ReaderEvent::LoadChapters => {
                        let book_id = self.book.as_ref().unwrap().id;

                        let (start, end) = self.get_next_pages_to_load();

                        if end != 0 {
                            ctx.link().send_future(async move {
                                Msg::RetrievePages(
                                    request::get_book_pages(book_id, start, end).await,
                                )
                            });
                        }
                    }

                    ReaderEvent::ViewOverlay(o_event) => {
                        if let OverlayEvent::Swipe {
                            type_of, instant, ..
                        } = o_event
                        {
                            if self.reader_settings.is_fullscreen && type_of == DragType::None {
                                if let Some(dur) = instant {
                                    if dur.num_milliseconds() < 500
                                        && self.display_toolbar.is_expanded()
                                    {
                                        self.display_toolbar = DisplayToolBars::Hidden;
                                        self.state.update_nav_visibility.emit(false);
                                    } else {
                                        self.display_toolbar = DisplayToolBars::Expanded;
                                        self.state.update_nav_visibility.emit(true);
                                    }
                                }
                            }
                        }
                    }
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
                    <div class={ book_class } style={ (self.display_toolbar.is_expanded() && self.reader_settings.is_fullscreen).then_some("transform: scale(0.8); height: 80%;") } ref={ self.ref_book_container.clone() }>
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
                            event={ ctx.link().callback(Msg::ReaderEvent) }
                        />
                    </div>

                    <div class={ classes!("tools", (self.reader_settings.is_fullscreen && !self.display_toolbar.is_expanded()).then_some("hidden")) }>
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

            ctx.link()
                .send_future(async move { Msg::RetrieveBook(request::get_book_info(id).await) });
        }

        // TODO: This is a duplicate of Msg::WindowResize
        if self.reader_settings.is_fullscreen
            && !self.display_toolbar.is_expanded()
            && !self.state.is_navbar_visible
        {
            if let Some(cont) = self.ref_book_container.cast::<Element>() {
                self.reader_settings.dimensions =
                    (cont.client_width().max(0), cont.client_height().max(0));
            }
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        if let Some(cb) = self.auto_resize_cb.take() {
            window()
                .remove_event_listener_with_callback("resize", cb.as_ref().unchecked_ref())
                .unwrap();
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

        window()
            .add_event_listener_with_callback("resize", handle_resize.as_ref().unchecked_ref())
            .unwrap();

        self.auto_resize_cb = Some(handle_resize);
    }

    // TODO: Use Option instead of returning (0, 0)
    fn get_next_pages_to_load(&self) -> (usize, usize) {
        let progress = self.progress.lock().unwrap();
        let chap_cont = self.chapters.lock().unwrap();

        let total_sections = self
            .book
            .as_ref()
            .map(|v| v.chapter_count)
            .unwrap_or_default();

        // Starting index
        let curr_section = if let Some(&Progression::Ebook { chapter, .. }) = progress.as_ref() {
            chapter as usize
        } else {
            0
        };

        let mut chapters = chap_cont
            .chapters
            .iter()
            .map(|v| v.value)
            .collect::<Vec<_>>();
        chapters.sort_unstable();

        match self.reader_settings.type_of {
            PageLoadType::All => {
                if chap_cont.chapters.is_empty() {
                    (curr_section.saturating_sub(2), curr_section + 3)
                } else {
                    let mut start_pos = 0;
                    let mut end_pos = 0;

                    // TODO: Simplify. Returns the next region of sections we need to load.

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
                    (curr_section.saturating_sub(2), curr_section + 3)
                } else {
                    // TODO: Simplify. Returns the next region of sections we need to load.

                    let found_previous =
                        curr_section != 0 && chapters.iter().any(|v| *v == curr_section - 1);
                    let found_next = curr_section + 1 != total_sections
                        && chapters.iter().any(|v| *v == curr_section + 1);

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
