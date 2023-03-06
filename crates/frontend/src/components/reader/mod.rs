use std::{path::PathBuf, rc::Rc, sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard}, time::Duration};

use common_local::{Chapter, MediaItem, Progression, MemberReaderPreferences, reader::ReaderColor};
use gloo_timers::callback::{Timeout, Interval};
use gloo_utils::{body, window};
use num_enum::{TryFromPrimitive, IntoPrimitive};
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast, UnwrapThrowExt,
};
use web_sys::{DomRect, Element, HtmlElement, HtmlIFrameElement};
use yew::{html::Scope, prelude::*};

use crate::{request, util::ElementEvent};

pub mod color;
pub mod layout;
pub mod section;
pub mod util;
pub mod view_overlay;

pub use self::layout::SectionDisplay;
use self::{section::{SectionContents, SectionLoadProgress}, layout::SectionDisplayType};
pub use self::view_overlay::{DragType, OverlayEvent, ViewOverlay};

const PAGE_CHANGE_DRAG_AMOUNT: usize = 200;

#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
    // TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
    fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;

    // TODO: Use Struct instead. Returns (byte position, section index)
    fn js_get_current_byte_pos(iframe: &HtmlIFrameElement, is_vertical: bool) -> Option<Vec<usize>>;
    fn js_get_page_from_byte_position(iframe: &HtmlIFrameElement, position: usize)
        -> Option<usize>;
    fn js_get_element_from_byte_position(
        iframe: &HtmlIFrameElement,
        position: usize,
    ) -> Option<HtmlElement>;

    fn js_update_iframe_after_load(
        iframe: &HtmlIFrameElement,
        chapter: &str,
        handle_js_redirect_clicks: &Closure<dyn FnMut(String, String)>,
    );

    fn js_get_visible_links(iframe: &HtmlIFrameElement, is_vscroll: bool) -> Vec<DomRect>;
}

macro_rules! get_current_section_mut {
    ($self:ident) => {{
        let hash = $self.cached_sections.get($self.viewing_section).map(|v| v.info.header_hash.as_str());

        $self.section_frames.iter_mut()
            .find_map(|sec| {
                let chap = sec.as_chapter_mut()?;

                if chap.header_hash.as_str() == hash? {
                    Some(chap)
                } else {
                    None
                }
            })
    }};
}

macro_rules! get_previous_section_mut {
    ($self:ident) => {{
        let sec_index = $self.get_current_frame_index();
        if let Some(prev_index) = sec_index.checked_sub(1) {
            $self.section_frames[prev_index].as_chapter_mut()
        } else {
            None
        }
    }};
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum PageLoadType {
    All = 0,
    #[default]
    Select,
}

#[derive(Clone)]
pub struct SharedReaderSettings(Rc<RwLock<ReaderSettings>>);

impl SharedReaderSettings {
    pub fn new(value: ReaderSettings) -> Self {
        Self(Rc::new(RwLock::new(value)))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, ReaderSettings> {
        self.0.read().unwrap_throw()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, ReaderSettings> {
        self.0.write().unwrap_throw()
    }
}

impl PartialEq for SharedReaderSettings {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Default, PartialEq)]
pub struct ReaderSettings {
    pub type_of: PageLoadType,
    pub color: ReaderColor,

    pub animate_page_transitions: bool,
    pub default_full_screen: bool,
    pub auto_full_screen: bool,
    pub display: SectionDisplay,
    pub show_progress: bool,

    pub dimensions: (i32, i32),
}

impl From<MemberReaderPreferences> for ReaderSettings {
    fn from(value: MemberReaderPreferences) -> Self {
        Self {
            type_of: PageLoadType::try_from(value.load_type).unwrap_throw(),
            color: value.color,
            animate_page_transitions: value.animate_page_transitions,
            default_full_screen: value.default_full_screen,
            auto_full_screen: value.auto_full_screen,
            display: SectionDisplay::from(value.display_type),
            show_progress: value.always_show_progress,
            dimensions: (value.width as i32, value.height as i32),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedPage {
    chapter: usize,
    chapter_local_page: usize,
}

// Currently used to load in chapters to the Reader.
#[derive(Clone)]
pub struct LoadedChapters {
    pub total: usize,
    pub chapters: Vec<Rc<Chapter>>,
}

impl LoadedChapters {
    pub fn new() -> Self {
        Self {
            total: 0,
            chapters: Vec::new(),
        }
    }
}

impl PartialEq for LoadedChapters {
    fn eq(&self, other: &Self) -> bool {
        self.total == other.total &&
        self.chapters.len() == other.chapters.len() &&
        self.chapters.iter().zip(other.chapters.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
    }
}

pub enum ReaderEvent {
    ViewOverlay(OverlayEvent),
}

#[derive(Properties)]
pub struct Property {
    // Callbacks
    pub event: Callback<ReaderEvent>,

    pub book: Rc<MediaItem>,
    pub chapters: LoadedChapters,

    pub progress: Rc<Mutex<Option<Progression>>>,
}

impl PartialEq for Property {
    fn eq(&self, other: &Self) -> bool {
        // self.event == other.event &&
        Rc::ptr_eq(&self.book, &other.book) &&
        self.chapters == other.chapters &&
        Rc::ptr_eq(&self.progress, &other.progress)
    }
}

pub enum ReaderMsg {
    SettingsUpdate(SharedReaderSettings),

    GenerateIFrameLoaded(usize),

    // Event
    HandleJsRedirect(String, String, Option<String>),

    PageTransitionStart,
    PageTransitionEnd,
    UpdateDragDistance,

    HandleScrollChangePage(DragType),
    HandleViewOverlay(OverlayEvent),
    UploadProgress,

    NextPage,
    PreviousPage,
    SetPage(usize),

    // TODO: Sections should be redefined as book chapters. We'll need to figure out where each chapter is.
    NextSection,
    PreviousSection,
    SetSection(usize),

    Ignore,
}

pub struct Reader {
    cached_dimensions: Option<(i32, i32)>,
    cached_sections: Vec<Rc<Chapter>>,

    // All the sections the books has and the current cached info
    section_frames: Vec<SectionLoadProgress>,

    /// The Chapter we're in
    viewing_section: usize,

    _interval: Interval,

    _handle_keyboard: ElementEvent,
    handle_js_redirect_clicks: Closure<dyn FnMut(String, String)>,
    cursor_type: &'static str,
    visible_redirect_rects: Vec<DomRect>,

    drag_distance: isize,

    scroll_change_page_timeout: Option<Timeout>,

    /// Are we switching Pages?
    is_transitioning: bool,

    initial_progression_set: bool,

    settings: SharedReaderSettings,
    _settings_listener: ContextHandle<SharedReaderSettings>
}

impl Component for Reader {
    type Message = ReaderMsg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let handle_js_redirect_clicks: Closure<dyn FnMut(String, String)> =
            Closure::new(move |section_hash: String, path: String| {
                let (file_path, id_value) = path
                    .split_once('#')
                    .map(|(a, b)| (a.to_string(), Some(b.to_string())))
                    .unwrap_or((path, None));

                link.send_message(ReaderMsg::HandleJsRedirect(section_hash, file_path, id_value));
            });

        let link = ctx.link().clone();
        let handle_keyboard: Closure<dyn FnMut(KeyboardEvent)> =
            Closure::new(move |event: KeyboardEvent| {
                if event.repeat() {
                    return;
                }

                match (event.shift_key(), event.code().as_str()) {
                    (false, "ArrowRight") => link.send_message(ReaderMsg::NextPage),
                    (false, "ArrowLeft") => link.send_message(ReaderMsg::PreviousPage),

                    (true, "ArrowRight") => link.send_message(ReaderMsg::NextSection),
                    (true, "ArrowLeft") => link.send_message(ReaderMsg::PreviousSection),
                    _ => (),
                }
            });

        let handle_keyboard = ElementEvent::link(
            window().unchecked_into(),
            handle_keyboard,
            |e, f| e.add_event_listener_with_callback("keydown", f),
            Box::new(|e, f| e.remove_event_listener_with_callback("keydown", f)),
        );

        let link = ctx.link().clone();
        let interval = Interval::new(
            10_000,
            move || link.send_message(ReaderMsg::UploadProgress)
        );

        let (settings, _settings_listener) = ctx
            .link()
            .context::<SharedReaderSettings>(ctx.link().callback(ReaderMsg::SettingsUpdate))
            .expect("context to be set");

        Self {
            cached_dimensions: None,
            cached_sections: Vec::new(),
            // Initialize with 1 section.
            section_frames: Vec::new(),
            // sections: (0..ctx.props().book.chapter_count)
            //     .map(|_| SectionLoadProgress::Waiting)
            //     .collect(),

            viewing_section: 0,
            drag_distance: 0,

            // TODO: Move both into own struct
            cursor_type: "default",
            visible_redirect_rects: Vec::new(),

            scroll_change_page_timeout: None,

            _interval: interval,

            handle_js_redirect_clicks,
            _handle_keyboard: handle_keyboard,
            is_transitioning: false,
            initial_progression_set: false,

            settings,
            _settings_listener,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ReaderMsg::Ignore => return false,

            ReaderMsg::SettingsUpdate(settings) => {
                self.settings = settings;
                return true;
            }

            ReaderMsg::PageTransitionStart => {
                // TODO FIX: Keyboard shortcut may some reason it may double.
                if self.is_transitioning {
                    return false;
                }

                debug!("Page Transition Start: {}", self.drag_distance);
                self.is_transitioning = true;
            }

            ReaderMsg::PageTransitionEnd => {
                // TODO FIX: Keyboard shortcut may some reason it may double.
                if !self.is_transitioning {
                    return false;
                }

                debug!("Page Transition End: {}", self.drag_distance);
                self.is_transitioning = false;

                // TODO: Check if we we changed pages to being with.

                self.after_page_change();

                // Page transitioning can happen on initial load of frames.
                // We have to make sure we've changed the page after the frame loaded.
                if self.initial_progression_set {
                    return Component::update(self, ctx, ReaderMsg::UploadProgress);
                } else {
                    return false;
                }
            }

            ReaderMsg::HandleJsRedirect(_section_hash, file_path, _id_name) => {
                debug!("ReaderMsg::HandleJsRedirect(section_hash: {_section_hash}, file_path: {file_path:?}, id_name: {_id_name:?})");

                let file_path = PathBuf::from(file_path);

                // TODO: Ensure we handle any paths which go to a parent directory. eg: "../file.html"
                // let mut path = chaps.chapters.iter().find(|v| v.value == chapter).unwrap().file_path.clone();
                // path.pop();

                if let Some(chap) = self.cached_sections
                    .iter()
                    .find(|v| v.file_path.ends_with(&file_path))
                    .cloned()
                {
                    self.set_section(chap.value, ctx);
                    // TODO: Handle id_name
                }
            }

            ReaderMsg::HandleViewOverlay(event) => {
                match event {
                    // Changes users' cursor if they're currently hovering over a redirect.
                    OverlayEvent::Hover { x, y } => {
                        if !self.settings.read().display.is_scroll() {
                            let (x, y) = (x as f64, y as f64);

                            let mut was_found = false;

                            for bb in &self.visible_redirect_rects {
                                if bb.x() <= x
                                    && bb.x() + bb.width() >= x
                                    && bb.y() <= y
                                    && bb.y() + bb.height() >= y
                                {
                                    if self.cursor_type == "default" {
                                        self.cursor_type = "pointer";
                                        body()
                                            .style()
                                            .set_property("cursor", self.cursor_type)
                                            .unwrap_throw();
                                    }

                                    was_found = true;

                                    break;
                                }
                            }

                            if !was_found && self.cursor_type != "default" {
                                self.cursor_type = "default";
                                body()
                                    .style()
                                    .set_property("cursor", self.cursor_type)
                                    .unwrap_throw();
                            }

                            return false;
                        }
                    }

                    OverlayEvent::Release { x, y, width, height, instant } => {
                        debug!("Input Release: [w{width}, h{height}], [x{x}, y{y}], took: {instant:?}");

                        let orig_drag_distance = self.drag_distance;

                        if self.drag_distance != 0 {
                            if self.drag_distance.unsigned_abs() > PAGE_CHANGE_DRAG_AMOUNT {
                                if self.drag_distance.is_positive() {
                                    return Component::update(self, ctx, ReaderMsg::PreviousPage);
                                } else {
                                    return Component::update(self, ctx, ReaderMsg::NextPage);
                                }
                            } else if let Some(section) = self.get_current_frame() {
                                self.settings.read().display.transitioning_page(0, section);
                                self.drag_distance = 0;
                            }
                            // Handle after dragging
                        }

                        // Handle Prev/Next Page Clicking
                        else if let Some(duration) = instant {
                            if !self.settings.read().display.is_scroll() && duration.to_std().unwrap_throw() < Duration::from_millis(800) {
                                let clickable_size = (width as f32 * 0.15) as i32;

                                // Previous Page
                                if x <= clickable_size {
                                    debug!("Clicked Previous");
                                    return Component::update(self, ctx, ReaderMsg::PreviousPage);
                                }

                                // Next Page
                                else if x >= width - clickable_size {
                                    debug!("Clicked Next");
                                    return Component::update(self, ctx, ReaderMsg::NextPage);
                                }
                            }
                        }

                        if orig_drag_distance == 0 {
                            ctx.props().event.emit(ReaderEvent::ViewOverlay(event));
                        }
                    }

                    OverlayEvent::Drag {
                        type_of,
                        instant,
                        coords_start,
                        ..
                    } => {
                        match type_of {
                            DragType::Up(_distance) => (),
                            DragType::Down(_distance) => (),

                            // Previous Page
                            DragType::Right(distance) => {
                                // TODO: Prevent drags from going past start
                                self.drag_distance = distance as isize;

                                if let Some(section) = self.get_current_frame() {
                                    self.settings.read().display.transitioning_page(self.drag_distance, section);
                                }
                            }

                            // Next Page
                            DragType::Left(distance) => {
                                // TODO: Prevent drags from going past end
                                self.drag_distance = -(distance as isize);

                                if let Some(section) = self.get_current_frame() {
                                    self.settings.read().display.transitioning_page(self.drag_distance, section);
                                }
                            }

                            DragType::None => {
                                // Clicked on a[href]
                                if let Some(dur) = instant {
                                    if dur.num_milliseconds() < 500 {
                                        if let Some(section) = self.get_current_frame() {
                                            let frame = section.get_iframe();
                                            let document =
                                                frame.content_document().unwrap_throw();
                                            let bb = frame.get_bounding_client_rect();

                                            let (x, y) = (
                                                coords_start.0 as f64 - bb.x(),
                                                coords_start.1 as f64 - bb.y(),
                                            );

                                            if let Some(element) =
                                                document.element_from_point(x as f32, y as f32)
                                            {
                                                fn contains_a_href(
                                                    element: Element,
                                                ) -> Option<HtmlElement>
                                                {
                                                    if element.local_name() == "a"
                                                        && element.has_attribute("href")
                                                    {
                                                        Some(element.unchecked_into())
                                                    } else if let Some(element) =
                                                        element.parent_element()
                                                    {
                                                        contains_a_href(element)
                                                    } else {
                                                        None
                                                    }
                                                }

                                                if let Some(element) = contains_a_href(element)
                                                {
                                                    element.click();
                                                    return false;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    OverlayEvent::Hold { since, x, y } => {
                        // debug!("{x} {y} {since:?}");
                    }
                }
            }

            ReaderMsg::HandleScrollChangePage(type_of) => {
                match type_of {
                    // Scrolling up
                    DragType::Up(_) => {
                        if self.viewing_section != 0 {
                            // TODO?: Ensure we've been stopped at the edge for at least 1 second before performing page change steps.
                            // Scrolling is split into 5 sections. You need to scroll up or down at least 3 time to change page to next after timeout.
                            // At 5 we switch automatically. It should also take 5 MAX to fill the current reader window.

                            let height = self.settings.read().dimensions.1 as isize / 5;

                            self.drag_distance += height;

                            if self.drag_distance / height == 5 {
                                self.drag_distance = 0;
                                self.previous_page();
                            } else {
                                // After 500 ms of no scroll activity reset position ( self.drag_distance ?? ) to ZERO.
                                let link = ctx.link().clone();
                                self.scroll_change_page_timeout =
                                    Some(Timeout::new(1_000, move || {
                                        link.send_message(ReaderMsg::UpdateDragDistance);
                                    }));
                            }
                        }
                    }

                    // Scrolling down
                    DragType::Down(_) => {
                        if self.viewing_section + 1 != self.cached_sections.len() {
                            let height = self.settings.read().dimensions.1 as isize / 5;

                            self.drag_distance -= height;

                            if self.drag_distance.abs() / height == 5 {
                                self.drag_distance = 0;
                                self.previous_page();
                            } else {
                                // After 500 ms of no scroll activity reset position ( self.drag_distance ?? ) to ZERO.
                                let link = ctx.link().clone();
                                self.scroll_change_page_timeout =
                                    Some(Timeout::new(1_000, move || {
                                        link.send_message(ReaderMsg::UpdateDragDistance);
                                    }));
                            }
                        }
                    }

                    _ => unreachable!(),
                }
            }

            ReaderMsg::UpdateDragDistance => {
                let height = self.settings.read().dimensions.1 as isize / 5;

                if self.drag_distance.abs() / height >= 3 {
                    if self.drag_distance.is_positive() {
                        self.drag_distance = 0;
                        self.previous_page();
                    } else {
                        self.drag_distance = 0;
                        self.next_page();
                    }
                } else {
                    self.drag_distance = 0;
                }
            }

            ReaderMsg::SetPage(new_page) => match { let v = self.settings.read().display.as_type(); v } {
                SectionDisplayType::Single | SectionDisplayType::Double => {
                    return self
                        .set_page(new_page.min(self.page_count(ctx).saturating_sub(1)));
                }

                SectionDisplayType::Scroll => {
                    return Component::update(self, ctx, ReaderMsg::SetSection(new_page));
                }

                SectionDisplayType::Image => {
                    return self
                        .set_page(new_page.min(self.page_count(ctx).saturating_sub(1)));
                }
            },

            ReaderMsg::NextPage => {
                match { let v = self.settings.read().display.as_type(); v } {
                    SectionDisplayType::Single | SectionDisplayType::Double => {
                        if self.current_page_pos() + 1 == self.page_count(ctx) {
                            return false;
                        }

                        self.next_page();
                    }

                    SectionDisplayType::Scroll => {
                        return Component::update(self, ctx, ReaderMsg::NextSection);
                    }

                    SectionDisplayType::Image => {
                        match self.settings.read().display.get_movement() {
                            layout::PageMovement::LeftToRight => {
                                if self.current_page_pos() + 1 == self.page_count(ctx) {
                                    return false;
                                }
                            }

                            layout::PageMovement::RightToLeft => {
                                if self.current_page_pos() == 0 {
                                    return false;
                                }
                            }
                        }

                        self.next_page();
                    }
                }

                self.drag_distance = 0;
            }

            ReaderMsg::PreviousPage => {
                match { let v = self.settings.read().display.as_type(); v } {
                    SectionDisplayType::Single | SectionDisplayType::Double => {
                        if self.current_page_pos() == 0 {
                            return false;
                        }

                        self.previous_page();
                    }

                    SectionDisplayType::Scroll => {
                        return Component::update(self, ctx, ReaderMsg::PreviousSection);
                    }

                    SectionDisplayType::Image => {
                        match self.settings.read().display.get_movement() {
                            layout::PageMovement::LeftToRight => {
                                if self.current_page_pos() == 0 {
                                    return false;
                                }
                            }

                            layout::PageMovement::RightToLeft => {
                                if self.current_page_pos() + 1 == self.page_count(ctx) {
                                    return false;
                                }
                            }
                        }

                        self.previous_page();
                    }
                }

                self.drag_distance = 0;
            }

            ReaderMsg::SetSection(new_section) => {
                if self.set_section(
                    new_section.min(ctx.props().book.chapter_count.saturating_sub(1)),
                    ctx,
                ) {
                    self.upload_progress_and_emit(ctx);

                    return true;
                } else {
                    // We couldn't set the chapter which means we have to load it.
                    // TODO: Should we do anything here? Chapter should be requested and starting to load at this point.
                }
            }

            ReaderMsg::NextSection => {
                let new_section_index = self.get_current_frame_index() + 1;

                if new_section_index == self.section_frames.len() {
                    return false;
                }

                let next_section = if let Some(sec) = self.section_frames[new_section_index].as_chapter() {
                    self.cached_sections.iter().position(|chap| chap.info.header_hash == sec.header_hash)
                } else {
                    None
                };

                self.set_section(next_section.unwrap_or(new_section_index), ctx);

                self.upload_progress_and_emit(ctx);

                self.drag_distance = 0;
            }

            ReaderMsg::PreviousSection => {
                let Some(new_section_index) = self.get_current_frame_index().checked_sub(1) else {
                    return false;
                };

                let next_section = if let Some(sec) = self.section_frames[new_section_index].as_chapter() {
                    self.cached_sections.iter()
                        .enumerate()
                        .rev()
                        .find_map(|(i, chap)| {
                            if chap.info.header_hash == sec.header_hash {
                                Some(i)
                            } else {
                                None
                            }
                        })
                } else {
                    None
                };

                self.set_section(next_section.unwrap_or(new_section_index), ctx);

                self.upload_progress_and_emit(ctx);

                self.drag_distance = 0;
            }

            ReaderMsg::UploadProgress => self.upload_progress_and_emit(ctx),

            // Called after iframe is loaded.
            ReaderMsg::GenerateIFrameLoaded(section_index) => {
                self.section_frames[section_index].convert_to_loaded();

                debug!("Generated Section Frame {}", section_index);

                // Call on_load for the newly loaded frame.
                if let SectionLoadProgress::Loaded(section) = &mut self.section_frames[section_index] {
                    section.on_load(
                        &self.handle_js_redirect_clicks,
                        &mut self.settings.write(),
                        ctx,
                    );
                }

                if self.are_all_sections_generated() {
                    self.on_all_frames_generated(ctx);
                }

                self.update_cached_pages(ctx.props());

                // TODO: Ensure this works.
                if self.settings.read().type_of == PageLoadType::Select {
                    self.use_progression(*ctx.props().progress.lock().unwrap(), ctx);
                }

                // Make sure the previous section is on the last page for better page turning on initial load.
                if let Some(prev_sect) = get_previous_section_mut!(self) {
                    self.settings.write().display.set_last_page(prev_sect);
                }
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let page_count = self.page_count(ctx);
        let section_count = ctx.props().book.chapter_count;

        let pages_style = format!(
            "width: {}px; height: {}px;",
            self.settings.read().dimensions.0,
            self.settings.read().dimensions.1
        );

        let progress_percentage = match self.settings.read().display {
            SectionDisplay::Double(_) | SectionDisplay::Single(_) => format!(
                "width: {}%;",
                (self.current_page_pos() + 1) as f64 / page_count as f64 * 100.0
            ),

            SectionDisplay::Scroll(_) => format!(
                "width: {}%;",
                (self.viewing_section + 1) as f64 / section_count as f64 * 100.0
            ),

            SectionDisplay::Image(_) => format!(
                "width: {}%;",
                (self.current_page_pos() + 1) as f64 / page_count as f64 * 100.0
            )
        };

        let (frame_class, frame_style) = self.get_frame_class_and_style();

        let link = ctx.link().clone();
        let link2 = ctx.link().clone();

        html! {
            <div class="reader">
                <div class="pages" style={ pages_style.clone() }>
                    {
                        if !self.settings.read().display.is_scroll() {
                            html! {
                                <ViewOverlay event={ ctx.link().callback(ReaderMsg::HandleViewOverlay) } />
                            }
                        } else {
                            html! {}
                        }
                    }
                    <div
                        class={ frame_class }
                        style={ frame_style }
                        // Frame changes use a transition.
                        ontransitionstart={ Callback::from(move|_| link2.send_message(ReaderMsg::PageTransitionStart)) }
                        ontransitionend={ Callback::from(move|_| link.send_message(ReaderMsg::PageTransitionEnd)) }
                    >
                        {
                            for (0..self.section_frames.len())
                                .into_iter()
                                .map(|i| {
                                    if let Some(v) = self.section_frames[i].as_chapter() {
                                        Html::VRef(v.get_iframe().clone().into())
                                    } else {
                                        html! {
                                            <div style={ pages_style.clone() }>
                                                <h2>{ format!("Loading Section #{i}") }</h2>
                                            </div>
                                        }
                                    }
                                })
                        }
                    </div>
                </div>

                { self.render_toolbar() }

                {
                    if self.settings.read().show_progress {
                        html! {
                            <div class="progress">
                                <div class="prog-bar" style={ progress_percentage }></div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _prev: &Self::Properties) -> bool {
        let props = ctx.props();

        self.cached_sections = props.chapters.chapters.clone();

        if let Some(Progression::Ebook { chapter, .. }) = *ctx.props().progress.lock().unwrap() {
            self.viewing_section = chapter as usize;
        }

        if self.cached_dimensions != Some(self.settings.read().dimensions) {
            self.cached_dimensions = Some(self.settings.read().dimensions);

            // Refresh all page styles and sizes.
            for prog in &self.section_frames {
                if let SectionLoadProgress::Loaded(section) = prog {
                    update_iframe_size(Some(self.settings.read().dimensions), section.get_iframe());

                    self.settings.write().display.add_to_iframe(section.get_iframe(), ctx);
                }
            }

            self.update_cached_pages(ctx.props());
        }

        self.load_surrounding_sections(ctx);

        // TODO: Ensure this works.
        if self.settings.read().type_of == PageLoadType::Select {
            self.use_progression(*props.progress.lock().unwrap(), ctx);
        }

        true
    }
}

impl Reader {
    fn load_surrounding_sections(&mut self, ctx: &Context<Self>) {
        debug!("Page Load Type: {:?}", self.settings.read().type_of);

        // TODO: Re-implement Select
        // match self.settings.read().type_of {
        //     PageLoadType::All => {
                let chapter_count = ctx.props().book.chapter_count;

                for chapter in 0..chapter_count {
                    self.load_section(chapter, ctx);
                }
        //     }

        //     PageLoadType::Select => {
        //         // Continue loading chapters
        //         let start = self.viewing_chapter.saturating_sub(2);

        //         for chapter in start..start + 5 {
        //             self.load_section(chapter, ctx);
        //         }
        //     }
        // }
    }

    fn render_toolbar(&self) -> Html {
        if self.settings.read().display.is_scroll() {
            html! {}
        } else {
            html! {
                <div class="toolbar">
                    <div class="d-flex w-100">
                        <span>{ "Page " } { self.current_page_pos() + 1 }</span>
                    </div>
                </div>
            }
        }
    }

    fn get_frame_class_and_style(&self) -> (&'static str, String) {
        let animate_page_transitions = self.settings.read().animate_page_transitions;

        let hash = self.cached_sections.get(self.viewing_section).map(|v| v.info.header_hash.as_str());

        let mut transition = Some("transition: left 0.5s ease 0s;").filter(|_| animate_page_transitions);

        if self.settings.read().display.is_scroll() {
            let viewing = self.section_frames.iter()
                .enumerate()
                .find_map(|(idx, sec)| {
                    if sec.as_chapter()?.header_hash.as_str() == hash? {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            (
                "frames",
                format!(
                    "top: calc(-{}% + {}px); {}",
                    viewing * 100,
                    self.drag_distance,
                    transition.unwrap_or_default()
                ),
            )
        } else {
            // Prevent empty pages when on the first or last page of a section.
            let amount = match self.settings.read().display.get_movement() {
                // If we're moving right to left, we're on the first page, and we're not on the last frame.
                layout::PageMovement::RightToLeft if self.drag_distance.is_negative() => {
                    if self
                        .get_current_frame()
                        .map(|v| self.settings.read().display.viewing_page(v) == 0)
                        .unwrap_or_default() &&
                        self.section_frames.len() > self.viewing_section + 1
                    {
                        self.drag_distance
                    } else {
                        transition = None;
                        0
                    }
                }

                layout::PageMovement::LeftToRight if self.drag_distance.is_positive() => {
                    if self
                        .get_current_frame()
                        .map(|v| self.settings.read().display.viewing_page(v) == 0)
                        .unwrap_or_default()
                    {
                        self.drag_distance
                    } else {
                        transition = None;
                        0
                    }
                }

                layout::PageMovement::RightToLeft if self.drag_distance.is_positive() => {
                    if self
                        .get_current_frame()
                        .map(|v| self.settings.read().display.viewing_page(v) == v.page_count().saturating_sub(1))
                        .unwrap_or_default()
                    {
                        self.drag_distance
                    } else {
                        transition = None;
                        0
                    }
                }

                layout::PageMovement::LeftToRight if self.drag_distance.is_negative() => {
                    if self
                        .get_current_frame()
                        .map(|v| self.settings.read().display.viewing_page(v) == v.page_count().saturating_sub(1))
                        .unwrap_or_default()
                    {
                        self.drag_distance
                    } else {
                        transition = None;
                        0
                    }
                }

                _ => 0,
            };

            let viewing = self.section_frames.iter()
                .enumerate()
                .find_map(|(idx, sec)| {
                    if sec.as_chapter()?.header_hash.as_str() == hash? {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            (
                "frames horizontal",
                format!(
                    "left: calc(-{}% + {}px); {}",
                    viewing * 100,
                    amount,
                    transition.unwrap_or_default()
                ),
            )
        }
    }

    fn use_progression(&mut self, prog: Option<Progression>, ctx: &Context<Self>) {
        // Only update progression if we're not changing pages/chapters.
        if self.is_transitioning {
            return;
        }

        debug!("use_progression: {:?}", prog);

        if let Some(prog) = prog {
            if let Progression::Ebook { chapter, char_pos, .. } = prog {
                let chapter = chapter as usize;

                if self.viewing_section != chapter {
                    debug!("use_progression - set section: {chapter}");

                    // TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
                    self.set_section(chapter, ctx);
                }

                if char_pos != -1 {
                    for sec in &mut self.section_frames {
                        if let SectionLoadProgress::Loaded(section) = sec {
                            if section.get_chapters().iter().any(|v| v.value == chapter) {
                                self.initial_progression_set = true;

                                if self.settings.read().display.is_scroll() {
                                    if let Some(element) = js_get_element_from_byte_position(
                                        section.get_iframe(),
                                        char_pos as usize,
                                    ) {
                                        element.scroll_into_view();
                                    }
                                } else {
                                    let page = js_get_page_from_byte_position(
                                        section.get_iframe(),
                                        char_pos as usize,
                                    );

                                    debug!("use_progression - set page: {:?}", page);

                                    if let Some(page) = page {
                                        self.settings.write().display.set_page(page, section);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            self.initial_progression_set = true;
        }
    }

    fn are_all_sections_generated(&self) -> bool {
        self.section_frames.iter().all(|v| v.is_loaded())
    }

    fn update_cached_pages(&mut self, props: &<Self as Component>::Properties) {
        let mut total_page_pos = 0;

        // TODO: Verify if needed. Or can we do values_mut() we need to have it in asc order
        for chapter in 0..self.section_frames.len() {
            if let SectionLoadProgress::Loaded(ele) = &mut self.section_frames[chapter] {
                let page_count = if props.book.is_comic_book() {
                    self.cached_sections.len()
                } else {
                    get_iframe_page_count(ele.get_iframe()).max(1)
                };


                ele.gpi = total_page_pos;

                total_page_pos += page_count;

                let mut items = Vec::new();

                for local_page in 0..page_count {
                    items.push(CachedPage {
                        chapter,
                        chapter_local_page: local_page,
                    });
                }

                ele.set_cached_pages(items);
            }
        }
    }

    fn on_all_frames_generated(&mut self, ctx: &Context<Self>) {
        info!("All Frames Generated");
        // Double check page counts before proceeding.
        self.update_cached_pages(ctx.props());

        // We set the page again to ensure we're transitioned on the correct page.
        // This is needed for RTL reading.
        self.set_page(0);

        // TODO: Move to Msg::GenerateIFrameLoaded so it's only in a single place.
        self.use_progression(*ctx.props().progress.lock().unwrap(), ctx);
    }

    fn next_page(&mut self) -> bool {
        let section_count = self.section_frames.len();

        let frame_index = self.get_current_frame_index();

        if let Some(curr_sect) = get_current_section_mut!(self) {
            if self.settings.write().display.next_page(curr_sect) {
                debug!("Next Page");
                return true;
            } else {
                self.settings.read().display.transitioning_page(0, curr_sect);
            }

            if frame_index + 1 != section_count {
                self.settings.read().display.on_stop_viewing(curr_sect);

                let (start, _) = self.get_frame_start_and_end_sections(frame_index + 1).unwrap();

                self.viewing_section = start;
                debug!("Next Section");

                // Make sure the next sections viewing page is zero.
                if let Some(next_sect) = get_current_section_mut!(self) {
                    let mut settings = self.settings.write();

                    settings.display.set_page(0, next_sect);
                    settings.display.on_start_viewing(next_sect);
                }

                // TODO: Disabled b/c of reader section generation rework.
                // self.load_surrounding_sections(ctx);

                return true;
            }
        }

        false
    }

    fn previous_page(&mut self) -> bool {
        let frame_index = self.get_current_frame_index();

        if let Some(curr_sect) = get_current_section_mut!(self) {
            if self.settings.write().display.previous_page(curr_sect) {
                debug!("Previous Page");
                return true;
            } else {
                self.settings.read().display.transitioning_page(0, curr_sect);
            }

            if frame_index != 0 {
                self.settings.read().display.on_stop_viewing(curr_sect);

                let (_, end) = self.get_frame_start_and_end_sections(frame_index - 1).unwrap();

                self.viewing_section = end;
                debug!("Previous Section");

                // Make sure the next sections viewing page is maxed.
                if let Some(next_sect) = get_current_section_mut!(self) {
                    let mut settings = self.settings.write();

                    settings.display.set_last_page(next_sect);
                    settings.display.on_start_viewing(next_sect);
                }

                // TODO: Disabled b/c of reader section generation rework.
                // self.load_surrounding_sections(ctx);

                return true;
            }
        }

        false
    }

    /// Expensive. Iterates through previous sections.
    fn set_page(&mut self, new_total_page: usize) -> bool {
        let mut settings = self.settings.write();

        for section_index in 0..self.section_frames.len() {
            if let SectionLoadProgress::Loaded(section) = &mut self.section_frames[section_index] {
                // This should only happen if the page isn't loaded for some reason.
                if new_total_page < section.gpi {
                    break;
                }

                let local_page = new_total_page - section.gpi;

                if local_page < section.page_count() {
                    self.viewing_section = section_index;

                    settings.display.set_page(local_page, section);

                    // TODO: Disabled b/c of reader section generation rework.
                    // self.load_surrounding_sections(ctx);

                    return true;
                }
            }
        }

        false
    }

    fn set_section(&mut self, next_section: usize, ctx: &Context<Self>) -> bool {
        let hash = self.cached_sections.get(next_section)
            .map(|v| v.info.header_hash.as_str());

        // Retrieve next section index and frame.
        let Some((next_section_index, next_section_frame)) = self.section_frames.iter()
            .enumerate()
            .find_map(|(i, sec)| {
                if let Some((chap, other_hash)) = sec.as_chapter().zip(hash) {
                    if chap.header_hash.as_str() == other_hash {
                        Some((i, sec))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) else {
                return false;
            };

        debug!("Change Section {next_section_index}");

        if next_section_frame.is_waiting() {
            info!("Next Section is not loaded - {}", next_section + 1);

            self.load_section(next_section, ctx);

            if let Some(Progression::Ebook {
                chapter,
                char_pos,
                page,
            }) = &mut *ctx.props().progress.lock().unwrap()
            {
                *chapter = next_section as i64;
                *char_pos = 0;
                *page = 0;
            }

            return false;
        }

        // Stop viewing current section.
        if let Some(section) = self.get_current_frame() {
            self.settings.read().display.on_stop_viewing(section);
        }

        if let SectionLoadProgress::Loaded(section) = &mut self.section_frames[next_section_index] {
            self.viewing_section = next_section;

            let mut settings = self.settings.write();

            settings.display.set_page(0, section);
            settings.display.on_start_viewing(section);

            // TODO: Disabled b/c of reader section generation rework.
            // self.load_surrounding_sections(ctx);

            true
        } else {
            false
        }
    }

    fn after_page_change(&mut self) {
        if let Some(section) = self.get_current_frame() {
            self.visible_redirect_rects =
                js_get_visible_links(section.get_iframe(), self.settings.read().display.is_scroll());
        }
    }

    /// Expensive. Iterates through sections backwards from last -> first.
    fn page_count(&self, ctx: &Context<Self>) -> usize {
        let section_count = ctx.props().book.chapter_count;

        for index in 1..=section_count {
            if let Some(pos) = self
                .section_frames
                .get(section_count - index)
                .and_then(|s| Some(s.as_loaded()?.get_page_count_until()))
            {
                return pos;
            }
        }

        0
    }

    fn current_page_pos(&self) -> usize {
        self.get_current_frame()
            .map(|s| s.gpi + self.settings.read().display.viewing_page(s))
            .unwrap_or_default()
    }

    fn get_current_frame(&self) -> Option<&SectionContents> {
        let hash = self.cached_sections.get(self.viewing_section)
            .map(|v| v.info.header_hash.as_str());

        self.section_frames.iter()
            .find_map(|sec| {
                let chap = sec.as_chapter()?;

                if chap.header_hash.as_str() == hash? {
                    Some(chap)
                } else {
                    None
                }
            })
    }

    fn get_frame_start_and_end_sections(&self, index: usize) -> Option<(usize, usize)> {
        let frame = self.section_frames.get(index)?.as_chapter()?;

        let mut start_index = None;

        for (i, sec) in self.cached_sections.iter().enumerate() {
            if start_index.is_none() {
                if frame.header_hash == sec.info.header_hash {
                    start_index = Some(i);
                }
            } else if frame.header_hash != sec.info.header_hash {
                return Some((start_index?, i - 1));
            }
        }

        Some((start_index?, self.cached_sections.len() - 1))
    }

    fn get_current_frame_index(&self) -> usize {
        let hash = self.cached_sections.get(self.viewing_section)
            .map(|v| v.info.header_hash.as_str());

        self.section_frames.iter()
            .enumerate()
            .find_map(|(index, sec)| {
                let chap = sec.as_chapter()?;

                if chap.header_hash.as_str() == hash? {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    // TODO: Move to SectionLoadProgress and combine into upload_progress
    fn upload_progress_and_emit(&mut self, ctx: &Context<Self>) {
        // Ensure our current chapter is fully loaded AND NOT loading.
        // FIX: For first load of the reader. js_get_current_byte_pos needs the frame body to be loaded. Otherwise error.
        // Could remove once we optimize the upload requests.
        if let Some(chap) = self
            .get_current_frame()
            .filter(|sec|
                self.section_frames.iter()
                .find_map(|v| if v.as_chapter()?.header_hash == sec.header_hash { Some(v.is_loaded()) } else { None })
                .unwrap_or_default())
        {
            // Clone to prevent immutable hold on self.
            let iframe = chap.get_iframe().clone();

            self.upload_progress(&iframe, ctx);
        }
    }

    // TODO: Move to SectionLoadProgress
    fn upload_progress(&mut self, iframe: &HtmlIFrameElement, ctx: &Context<Self>) {
        let (chapter, page, char_pos, book_id) = {
            let (char_pos, viewing_section) = js_get_current_byte_pos(iframe, self.settings.read().display.is_scroll())
                .map(|v| (v[0] as i64, v[1]))
                .unwrap_or((-1, self.viewing_section));

            self.viewing_section = viewing_section;

            (
                viewing_section,
                self.get_current_frame()
                    .map(|v| self.settings.read().display.viewing_page(v))
                    .unwrap_or_default() as i64,
                char_pos,
                ctx.props().book.id,
            )
        };

        let last_page = self.page_count(ctx).saturating_sub(1);

        let stored_prog = Rc::clone(&ctx.props().progress);

        let req = match self.page_count(ctx) {
            0 if chapter == 0 => {
                *stored_prog.lock().unwrap() = None;

                None
            }

            // TODO: Figure out what the last page of each book actually is.
            v if v == last_page && chapter == self.section_frames.len().saturating_sub(1) => {
                let value = Some(Progression::Complete);

                *stored_prog.lock().unwrap() = value;

                value
            }

            _ => {
                let value = Some(Progression::Ebook {
                    char_pos,
                    chapter: chapter as i64,
                    page,
                });

                let mut stored_prog = stored_prog.lock().unwrap();

                if *stored_prog == value {
                    return;
                }

                *stored_prog = value;

                value
            }
        };

        ctx.link().send_future(async move {
            match req {
                Some(req) => request::update_book_progress(book_id, &req).await,
                None => request::remove_book_progress(book_id).await,
            };

            ReaderMsg::Ignore
        });
    }

    // TODO: Move to SectionLoadProgress
    // fn refresh_section(&mut self, chapter: usize, ctx: &Context<Self>) {
    //     let chaps = ctx.props().chapters.lock().unwrap();
    //
    //     if chaps.chapters.len() <= chapter {
    //         return;
    //     }
    //
    //     let chap = chaps.chapters[chapter].clone();
    //
    //     let sec = &mut self.sections[chap.value];
    //
    //     if let SectionLoadProgress::Loaded(sec) = sec {
    //         sec.get_iframe().remove();
    //     }
    //
    //     *sec = SectionLoadProgress::Loading(generate_section(
    //         Some(self.settings.read().dimensions),
    //         ctx.props().book.id,
    //         chap,
    //         ctx.link().clone(),
    //     ));
    // }

    // TODO: Move to SectionLoadProgress
    fn load_section(&mut self, chapter: usize, ctx: &Context<Self>) {
        if self.cached_sections.len() <= chapter {
            return;
        }

        // TODO: Load based on prev/next chapters instead of last section.
        let curr_chap = &self.cached_sections[chapter];

        // Check if chapter is already in section, return.
        // FIX: If the Properties changes this would be called again.
        {
            for sec in self.section_frames.iter().filter_map(|v| v.as_chapter()) {
                if sec.get_chapters().iter().any(|v| v.value == curr_chap.value) {
                    return;
                }
            }
        }

        let section_index = self.section_frames.len();

        // Create or append section.
        let use_last_section = self.section_frames.last()
            .and_then(|v| Some(v.as_chapter()?.header_hash == curr_chap.info.header_hash))
            .unwrap_or_default();

        if let Some(section_frame) = use_last_section.then_some(self.section_frames.last_mut()).flatten() {
            if section_frame.is_waiting() {
                info!("Generating Section {}", curr_chap.value + 1);

                *section_frame = SectionLoadProgress::Loading(generate_section(
                    Some(self.settings.read().dimensions),
                    curr_chap.info.header_hash.clone(),
                    section_index,
                    ctx.link().clone(),
                ));
            }
        } else {
            self.section_frames.push(SectionLoadProgress::Loading(generate_section(
                Some(self.settings.read().dimensions),
                curr_chap.info.header_hash.clone(),
                section_index,
                ctx.link().clone(),
            )));
        }

        // If last section was loaded.
        match self.section_frames.last_mut() {
            Some(SectionLoadProgress::Loaded(_contents)) => {
                // TODO: Insert into frame and update render.
                // TODO: To update we'll have to implement element boundary updates.
            }

            Some(SectionLoadProgress::Loading(contents)) => {
                contents.append_chapter(curr_chap.clone());
            }

            _ => ()
        }
    }
}

fn create_iframe() -> HtmlIFrameElement {
    gloo_utils::document()
        .create_element("iframe")
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn generate_section(
    book_dimensions: Option<(i32, i32)>,
    header_hash: String,
    section_index: usize,
    scope: Scope<Reader>,
) -> SectionContents {
    // TODO: Rework how we handle sections.
    // Join sections with the same stylesheets into one.
    // If next section is missing a stylesheet we'll join into previous iframe.
    let iframe = create_iframe();

    iframe.set_attribute("fetchPriority", "low").unwrap();
    iframe.set_attribute("data-hash", &header_hash).unwrap();

    // iframe
    //     .set_attribute(
    //         "src",
    //         &request::compile_book_resource_path(
    //             book_id,
    //             &chapter.file_path,
    //             api::LoadResourceQuery {
    //                 configure_pages: true,
    //             },
    //         ),
    //     )
    //     .unwrap();

    update_iframe_size(book_dimensions, &iframe);

    let f = Closure::wrap(Box::new(move || {
        scope.send_message(ReaderMsg::GenerateIFrameLoaded(section_index));
    }) as Box<dyn FnMut()>);

    iframe.set_onload(Some(f.as_ref().unchecked_ref()));

    SectionContents::new(header_hash, iframe, f)
}

fn update_iframe_size(book_dimensions: Option<(i32, i32)>, iframe: &HtmlIFrameElement) {
    let (width, height) = match book_dimensions {
        // TODO: Use Option.unzip once stable.
        Some((a, b)) => (a, b),
        None => (
            gloo_utils::body().client_width().max(0),
            gloo_utils::body().client_height().max(0),
        ),
    };

    iframe
        .style()
        .set_property("width", &format!("{width}px"))
        .unwrap();
    iframe
        .style()
        .set_property("height", &format!("{height}px"))
        .unwrap();
}
