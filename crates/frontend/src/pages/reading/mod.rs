// TODO: Handle resizing.

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use common::{
    api::WrappingResponse,
};
use common_local::{
    api::{self, GetChaptersResponse},
    FileId, MediaItem, Progression,
};
use gloo_timers::callback::Timeout;
use gloo_utils::{window, body};
use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
use web_sys::Element;
use yew::{context::ContextHandle, prelude::*};

use crate::{
    components::reader::{
        LoadedChapters, ReaderEvent, ReaderSettings, OverlayEvent, Reader, LayoutDisplay, layout::PageMovement, SharedReaderSettings
    },
    request, AppState, util::ElementEvent,
};


pub enum Msg {
    // Event
    WindowResize,

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

    reader_settings: SharedReaderSettings,
    progress: Rc<Mutex<Option<Progression>>>,
    book: Option<Rc<MediaItem>>,
    chapters: LoadedChapters,
    last_grabbed_count: usize,
    // TODO: Cache pages
    auto_resize_cb: Option<Closure<dyn FnMut()>>,

    timeout: Option<Timeout>,

    _on_fullscreen_event: ElementEvent,

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

        let mut reader_settings = ReaderSettings::from(state.member.as_ref().unwrap().parse_preferences().unwrap().unwrap_or_default().desktop.reader);

        let (win_width, win_height) = (
            window().inner_width().unwrap_throw().as_f64().unwrap(),
            window().inner_height().unwrap_throw().as_f64().unwrap(),
        );

        // Full screen the reader if our screen size is too small or we automatically do it.
        if reader_settings.auto_full_screen || win_width < 1200.0 || win_height < 720.0 {
            state.update_nav_visibility.emit(false);

            reader_settings.default_full_screen = true;
        }

        let on_fullscreen_event = {
            let link = ctx.link().clone();
            let function: Closure<dyn FnMut(Event)> =
                Closure::new(move |_: Event| link.send_message(Msg::WindowResize));

            ElementEvent::link(
                body().unchecked_into(),
                function,
                |e, f| e.add_event_listener_with_callback("fullscreenchange", f),
                Box::new(|e, f| e.remove_event_listener_with_callback("fullscreenchange", f)),
            )
        };

        Self {
            state,
            _listener,

            reader_settings: SharedReaderSettings::new(reader_settings),
            chapters: LoadedChapters::new(),
            last_grabbed_count: 0,
            progress: Rc::new(Mutex::new(None)),
            book: None,

            auto_resize_cb: None,

            timeout: None,

            _on_fullscreen_event: on_fullscreen_event,

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
                if self.reader_settings.default_full_screen
                    && !self.state.is_navbar_visible
                {
                    // FIX: Using "if let" since container can be null if this is called before first render.
                    if let Some(cont) = self.ref_book_container.cast::<Element>() {
                        Rc::get_mut(&mut self.reader_settings.0).unwrap().dimensions =
                            (cont.client_width().max(0), cont.client_height().max(0));

                        debug!("Window Resize: {:?}", self.reader_settings.dimensions);
                    } else {
                        debug!("Window Resize: book container doesn't exist");
                    }
                } else {
                    return false;
                }
            }

            Msg::RetrievePages(resp) => match resp.ok() {
                Ok(info) => {
                    self.last_grabbed_count = info.limit;
                    self.chapters.total = info.total;

                    for item in info.items {
                        self.chapters.chapters.push(Rc::new(item));
                    }
                }

                Err(err) => crate::display_error(err),
            },

            Msg::RetrieveBook(resp) => match resp.ok() {
                Ok(Some(resp)) => {
                    // Get Chapters.

                    let file_id = resp.media.id;

                    let end = resp.media.chapter_count;

                    if end != 0 {
                        ctx.link().send_future(async move {
                            Msg::RetrievePages(request::get_book_pages(file_id, 0, end).await)
                        });
                    }

                    // TODO: Remove this once we have a better way to handle this.
                    if resp.media.is_comic_book() {
                        Rc::get_mut(&mut self.reader_settings.0).unwrap().display = LayoutDisplay::new_image(PageMovement::RightToLeft);
                    }

                    self.book = Some(Rc::new(resp.media));
                    *self.progress.lock().unwrap() = resp.progress;
                }

                Ok(None) => (),

                Err(err) => crate::display_error(err),
            },

            Msg::ReaderEvent(event) => {
                match event {
                    ReaderEvent::ViewOverlay(o_event) => {
                        if let OverlayEvent::Release {
                            instant, ..
                        } = o_event
                        {
                            if self.reader_settings.default_full_screen {
                                // TODO: This is causing yew to "reload" the page.
                                if let Some(dur) = instant {
                                    self.state.update_nav_visibility.emit(dur.num_milliseconds() > 500);
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

            if self.reader_settings.default_full_screen {
                book_class += " overlay-x overlay-y";
            }

            let style = (self.reader_settings.default_full_screen && self.state.is_navbar_visible).then_some("transform: scale(0.8); height: 80%;");

            // TODO: Loading screen until sections have done initial generation.

            html! {
                <div class="reading-container">
                    <div class={ book_class } {style} ref={ self.ref_book_container.clone() }>
                        <ContextProvider<SharedReaderSettings> context={ self.reader_settings.clone() }>
                            <Reader
                                progress={ Rc::clone(&self.progress) }
                                book={ Rc::clone(book) }
                                chapters={ self.chapters.clone() }
                                event={ ctx.link().callback(Msg::ReaderEvent) }
                            />
                        </ContextProvider<SharedReaderSettings>>
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
        if self.reader_settings.default_full_screen && !self.state.is_navbar_visible
        {
            if let Some(cont) = self.ref_book_container.cast::<Element>() {
                Rc::get_mut(&mut self.reader_settings.0).unwrap().dimensions =
                    (cont.client_width().max(0), cont.client_height().max(0));

                debug!("Render Size: {:?}", self.reader_settings.dimensions);
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
}
