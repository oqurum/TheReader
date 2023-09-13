use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use common::api::WrappingResponse;
use common_local::{
    api::{self, GetChaptersResponse},
    FileId,
};
use gloo_events::EventListener;
use gloo_timers::callback::Timeout;
use gloo_utils::{body, window};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::Element;
use yew::{context::ContextHandle, prelude::*};

use crate::{
    components::reader::{
        layout::PageMovement, navbar::ReaderNavbar, LayoutDisplay, LoadedChapters, OverlayEvent,
        Reader, ReaderEvent, ReaderSettings, ReadingInfo, SharedInnerReaderSettings,
        SharedReaderSettings, UpdatableReadingInfo,
    },
    get_preferences, request,
    util::is_mobile_or_tablet,
    AppState,
};

static IS_FULL_SCREEN: AtomicBool = AtomicBool::new(false);

pub fn set_full_screen(value: bool) {
    IS_FULL_SCREEN.store(value, Ordering::Relaxed);
}

pub fn is_full_screen() -> bool {
    IS_FULL_SCREEN.load(Ordering::Relaxed)
}

pub enum Msg {
    // Event
    WindowResize,

    // Send
    ReaderEvent(ReaderEvent),

    // Retrieve
    RetrieveBook(WrappingResponse<api::GetBookResponse>),
    RetrieveBookFile(WrappingResponse<api::ApiGetFileByIdResponse>),
    RetrievePages(WrappingResponse<GetChaptersResponse>),

    ContextChanged(Rc<AppState>),
    ReadingInfoUpdated(()),
}

#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub id: FileId,
}

pub struct ReadingBook {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    // Book Dimensions
    book_dimensions: (i32, i32),

    reader_settings: SharedReaderSettings,
    reading_info: UpdatableReadingInfo,

    last_grabbed_count: usize,
    // TODO: Cache pages
    timeout: Option<Timeout>,

    _on_fullscreen_event: EventListener,
    _on_resize_event: EventListener,

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

        let reader_settings = {
            let prefs = get_preferences().unwrap().unwrap_or_default();

            SharedReaderSettings::new(SharedInnerReaderSettings {
                general: ReaderSettings::from(prefs.text_book.desktop.general),
                text: None,
                image: None,
            })
        };

        let (win_width, win_height) = (
            window().inner_width().unwrap_throw().as_f64().unwrap(),
            window().inner_height().unwrap_throw().as_f64().unwrap(),
        );

        state.update_nav_visibility.emit(false);

        // Full screen the reader if our screen size is too small or we automatically do it.
        // TODO: We'll have custom reader widths. Remove these hardcoded ones.
        if reader_settings.auto_full_screen || win_width < 1200.0 || win_height < 720.0 {
            set_full_screen(true);
        } else {
            set_full_screen(false);
        }

        Self {
            state,
            _listener,

            book_dimensions: reader_settings.dimensions,

            reader_settings,
            reading_info: UpdatableReadingInfo::new(
                ReadingInfo {
                    chapters: LoadedChapters::new(),
                    progress: None,
                    table_of_contents: Vec::new(),
                    file: None,
                    book: None,
                },
                ctx.link().callback(Msg::ReadingInfoUpdated),
            ),
            last_grabbed_count: 0,

            _on_resize_event: {
                let link = ctx.link().clone();
                let timeout: Arc<Mutex<Option<Timeout>>> = Arc::new(Mutex::new(None));

                EventListener::new(&window(), "resize", move |_: &Event| {
                    let link = link.clone();

                    let timeout_cloned = timeout.clone();

                    drop(timeout_cloned.lock().unwrap().take());

                    let to = Timeout::new(250, move || {
                        link.send_message(Msg::WindowResize);
                    });

                    *timeout_cloned.lock().unwrap() = Some(to);
                })
            },

            timeout: None,

            _on_fullscreen_event: {
                let link = ctx.link().clone();

                EventListener::new(&body(), "fullscreenchange", move |_: &Event| {
                    link.send_message(Msg::WindowResize)
                })
            },

            ref_book_container: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ReadingInfoUpdated(_) => {
                self.reading_info.set_updated();
            }

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
                if is_full_screen() {
                    // FIX: Using "if let" since container can be null if this is called before first render.
                    if let Some(cont) = self.ref_book_container.cast::<Element>() {
                        self.book_dimensions =
                            (cont.client_width().max(0), cont.client_height().max(0));

                        debug!("Window Resize: {:?}", self.book_dimensions);
                    } else {
                        debug!("Window Resize: book container doesn't exist");
                    }
                } else if self.reader_settings.display.is_scroll() {
                    if let Some(cont) = self.ref_book_container.cast::<Element>() {
                        self.book_dimensions.1 = cont.client_height().saturating_sub(20).max(0);

                        debug!("Window Resize: {:?}", self.book_dimensions);
                    }
                } else {
                    // TODO: Fullscreen once we're so small -- also have to re-adjust location
                    return false;
                }
            }

            Msg::RetrievePages(resp) => match resp.ok() {
                Ok(info) => {
                    self.last_grabbed_count = info.limit;

                    self.reading_info.update(|store| {
                        store.chapters.total = info.total;

                        for item in info.items {
                            store.chapters.chapters.push(Rc::new(item));
                        }
                    });
                }

                Err(err) => crate::display_error(err),
            },

            Msg::RetrieveBook(resp) => match resp.ok() {
                Ok(resp) => {
                    self.reading_info.update(|store| {
                        store.book = Some(resp.book);
                    });
                }

                Err(err) => crate::display_error(err),
            },

            Msg::RetrieveBookFile(resp) => match resp.ok() {
                Ok(Some(mut resp)) => {
                    // Get Chapters.

                    let file_id = resp.media.id;

                    let end = resp.media.chapter_count;

                    if end != 0 {
                        ctx.link().send_future(async move {
                            Msg::RetrievePages(request::get_book_pages(file_id, 0, end).await)
                        });
                    }

                    {
                        let Some(book_id) = resp.media.book_id else {
                            crate::display_error(common::api::ApiErrorResponse::new(
                                "No Book affiliated w/ file",
                            ));
                            return false;
                        };

                        ctx.link().send_future(async move {
                            Msg::RetrieveBook(request::get_media_view(book_id).await)
                        });
                    }

                    self.reading_info.update(|store| {
                        store.file = Some(resp.media);
                        store.progress = resp.progress;

                        resp.toc.sort_unstable_by(|a, b| a.1.cmp(&b.1));
                        store.table_of_contents = resp.toc;
                    });

                    self.update_settings();
                }

                Ok(None) => (),

                Err(err) => crate::display_error(err),
            },

            Msg::ReaderEvent(event) => {
                match event {
                    ReaderEvent::ViewOverlay(o_event) => {
                        if let OverlayEvent::Release { instant, .. } = o_event {
                            if is_full_screen() {
                                // TODO: This is causing yew to "reload" the page.
                                if let Some(dur) = instant {
                                    self.state
                                        .update_nav_visibility
                                        .emit(dur.num_milliseconds() > 500);
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
        {
            let info = self.reading_info.borrow();

            if info.file.is_none() || info.book.is_none() {
                return html! {
                    <h1>{ "Loading..." }</h1>
                };
            }
        }

        let mut book_class = String::from("book");

        if is_full_screen() {
            book_class += " overlay-x overlay-y";
        }

        html! {
            <div class="reading-container">
                <ContextProvider<UpdatableReadingInfo> context={ self.reading_info.clone() }>
                    {
                        if is_full_screen() {
                            html! {}
                        } else {
                            html! {
                                <ReaderNavbar />
                            }
                        }
                    }

                    <div class={ book_class } ref={ self.ref_book_container.clone() }>
                        <ContextProvider<SharedReaderSettings> context={ self.reader_settings.clone() }>
                            <Reader
                                width={ self.book_dimensions.0 }
                                height={ self.book_dimensions.1 }

                                event={ ctx.link().callback(Msg::ReaderEvent) }
                            />
                        </ContextProvider<SharedReaderSettings>>
                    </div>
                </ContextProvider<UpdatableReadingInfo>>
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let id = ctx.props().id;

            ctx.link().send_future(async move {
                Msg::RetrieveBookFile(request::get_book_info(id).await)
            });
        }

        // TODO: This is a duplicate of Msg::WindowResize
        if is_full_screen() {
            if let Some(cont) = self.ref_book_container.cast::<Element>() {
                // TODO: Remove. We don't want to update the settings. It should be immutable.
                self.book_dimensions = (cont.client_width().max(0), cont.client_height().max(0));

                debug!("Render Size: {:?}", self.book_dimensions);
            }
        } else if self.reader_settings.display.is_scroll() {
            if let Some(cont) = self.ref_book_container.cast::<Element>() {
                self.book_dimensions.1 = cont.client_height().saturating_sub(20).max(0);

                debug!("Render Size: {:?}", self.book_dimensions);
            }
        }
    }
}

impl ReadingBook {
    fn update_settings(&mut self) {
        let info = self.reading_info.borrow();
        let Some(book) = info.file.as_ref() else {
            return;
        };

        let prefs = get_preferences().unwrap().unwrap_or_default();

        let (general, text, image) = match (book.is_comic_book(), is_mobile_or_tablet()) {
            (true, true) => (
                prefs.image_book.mobile.general,
                None,
                Some(prefs.image_book.mobile.image),
            ),
            (true, false) => (
                prefs.image_book.desktop.general,
                None,
                Some(prefs.image_book.desktop.image),
            ),
            (false, true) => (
                prefs.text_book.mobile.general,
                Some(prefs.text_book.mobile.reader),
                None,
            ),
            (false, false) => (
                prefs.text_book.desktop.general,
                Some(prefs.text_book.desktop.reader),
                None,
            ),
        };

        let mut general = ReaderSettings::from(general);

        // TODO: Remove this once we have a better way to handle this.
        if book.is_comic_book() {
            general.display = LayoutDisplay::new_image(PageMovement::RightToLeft);
        }

        self.reader_settings = SharedReaderSettings::new(SharedInnerReaderSettings {
            general,
            text,
            image,
        });
    }
}
