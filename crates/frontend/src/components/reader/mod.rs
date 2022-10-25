use std::{path::PathBuf, rc::Rc, sync::Mutex};

use common_local::{api, Chapter, FileId, MediaItem, Progression};
use gloo_timers::callback::Timeout;
use gloo_utils::body;
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast, UnwrapThrowExt,
};
use web_sys::{DomRect, Element, HtmlElement, HtmlIFrameElement};
use yew::{html::Scope, prelude::*};

use crate::request;

pub mod color;
pub mod layout;
pub mod section;
pub mod util;
pub mod view_overlay;

pub use self::layout::SectionDisplay;
use self::section::{SectionContents, SectionLoadProgress};
pub use self::view_overlay::{DragType, OverlayEvent, ViewOverlay};

const PAGE_CHANGE_DRAG_AMOUNT: usize = 200;

#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
    // TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
    fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;

    fn js_get_current_byte_pos(iframe: &HtmlIFrameElement) -> Option<usize>;
    fn js_get_page_from_byte_position(iframe: &HtmlIFrameElement, position: usize)
        -> Option<usize>;
    fn js_get_element_from_byte_position(
        iframe: &HtmlIFrameElement,
        position: usize,
    ) -> Option<HtmlElement>;

    fn js_update_iframe_after_load(
        iframe: &HtmlIFrameElement,
        chapter: usize,
        handle_js_redirect_clicks: &Closure<dyn FnMut(usize, String)>,
    );

    fn js_get_visible_links(iframe: &HtmlIFrameElement, is_vscroll: bool) -> Vec<DomRect>;
}

macro_rules! get_current_section_mut {
    ($self:ident) => {
        $self
            .sections
            .get_mut($self.viewing_chapter)
            .and_then(|v| v.as_chapter_mut())
    };
}

macro_rules! get_previous_section_mut {
    ($self:ident) => {
        if let Some(chapter) = $self.viewing_chapter.checked_sub(1) {
            $self
                .sections
                .get_mut(chapter)
                .and_then(|v| v.as_chapter_mut())
        } else {
            None
        }
    };
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PageLoadType {
    All,
    #[default]
    Select,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReaderSettings {
    pub load_speed: usize,
    pub type_of: PageLoadType,

    pub is_fullscreen: bool,
    pub display: SectionDisplay,
    pub show_progress: bool,

    pub dimensions: (i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedPage {
    chapter: usize,
    chapter_local_page: usize,
}

// Currently used to load in chapters to the Reader.
pub struct LoadedChapters {
    pub total: usize,
    pub chapters: Vec<Chapter>,
}

impl LoadedChapters {
    pub fn new() -> Self {
        Self {
            total: 0,
            chapters: Vec::new(),
        }
    }
}

pub enum ReaderEvent {
    ViewOverlay(OverlayEvent),
}

#[derive(Properties)]
pub struct Property {
    pub settings: ReaderSettings,

    // Callbacks
    pub event: Callback<ReaderEvent>,

    pub book: Rc<MediaItem>,
    pub chapters: Rc<Mutex<LoadedChapters>>,

    pub progress: Rc<Mutex<Option<Progression>>>,
}

impl PartialEq for Property {
    fn eq(&self, _other: &Self) -> bool {
        // TODO
        false
    }
}

pub enum ReaderMsg {
    GenerateIFrameLoaded(Chapter),

    // Event
    HandleJsRedirect(usize, String, Option<String>),

    PageTransitionEnd,
    UpdateDragDistance,

    HandleScrollChangePage(DragType),
    HandleViewOverlay(OverlayEvent),
    UploadProgress,

    NextPage,
    PreviousPage,
    SetPage(usize),

    Ignore,
}

pub struct Reader {
    // Cached from External Source
    // TODO: Should I cache it?
    cached_display: SectionDisplay,
    cached_dimensions: Option<(i32, i32)>,

    // All the sections the books has and the current cached info
    sections: Vec<SectionLoadProgress>,

    /// The Chapter we're in
    viewing_chapter: usize,

    handle_js_redirect_clicks: Closure<dyn FnMut(usize, String)>,
    cursor_type: &'static str,
    visible_redirect_rects: Vec<DomRect>,

    drag_distance: isize,

    scroll_change_page_timeout: Option<Timeout>,
}

impl Component for Reader {
    type Message = ReaderMsg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let handle_js_redirect_clicks =
            Closure::wrap(Box::new(move |chapter: usize, path: String| {
                let (file_path, id_value) = path
                    .split_once('#')
                    .map(|(a, b)| (a.to_string(), Some(b.to_string())))
                    .unwrap_or((path, None));

                link.send_message(ReaderMsg::HandleJsRedirect(chapter, file_path, id_value));
            }) as Box<dyn FnMut(usize, String)>);

        Self {
            cached_display: ctx.props().settings.display.clone(),
            cached_dimensions: None,
            sections: (0..ctx.props().book.chapter_count)
                .map(|_| SectionLoadProgress::Waiting)
                .collect(),

            viewing_chapter: 0,
            drag_distance: 0,

            cursor_type: "default",
            visible_redirect_rects: Vec::new(),

            scroll_change_page_timeout: None,

            handle_js_redirect_clicks,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ReaderMsg::Ignore => return false,

            ReaderMsg::PageTransitionEnd => {
                // TODO: Check if we we changed pages to being with.

                log::info!("transition");

                self.after_page_change();

                return self.update(ctx, ReaderMsg::UploadProgress);
            }

            ReaderMsg::HandleJsRedirect(_chapter, file_path, _id_name) => {
                log::debug!("ReaderMsg::HandleJsRedirect(chapter: {_chapter}, file_path: {file_path:?}, id_name: {_id_name:?})");

                let file_path = PathBuf::from(file_path);

                let chaps = ctx.props().chapters.lock().unwrap();

                // TODO: Ensure we handle any paths which go to a parent directory. eg: "../file.html"
                // let mut path = chaps.chapters.iter().find(|v| v.value == chapter).unwrap().file_path.clone();
                // path.pop();

                if let Some(chap) = chaps
                    .chapters
                    .iter()
                    .find(|v| v.file_path.ends_with(&file_path))
                    .cloned()
                {
                    drop(chaps);
                    self.set_section(chap.value, ctx);
                    // TODO: Handle id_name
                }
            }

            ReaderMsg::HandleViewOverlay(event) => {
                match event {
                    // Changes users' cursor if they're currently hovering over a redirect.
                    OverlayEvent::MouseMove { x, y } => {
                        if !self.cached_display.is_scroll() {
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

                    OverlayEvent::Swipe {
                        type_of,
                        dragging,
                        instant,
                        coords_start,
                        ..
                    } => {
                        match type_of {
                            DragType::Up(_distance) => (),
                            DragType::Down(_distance) => (),

                            // Previous Page
                            DragType::Right(distance) => {
                                if dragging {
                                    self.drag_distance = distance as isize;

                                    if let Some(section) = self.get_current_section() {
                                        section.transitioning_page(self.drag_distance);
                                    }
                                } else if distance > PAGE_CHANGE_DRAG_AMOUNT {
                                    return self.update(ctx, ReaderMsg::PreviousPage);
                                } else if let Some(section) = self.get_current_section() {
                                    section.transitioning_page(0);
                                    self.drag_distance = 0;
                                }
                            }

                            // Next Page
                            DragType::Left(distance) => {
                                if dragging {
                                    self.drag_distance = -(distance as isize);

                                    if let Some(section) = self.get_current_section() {
                                        section.transitioning_page(self.drag_distance);
                                    }
                                } else if distance > PAGE_CHANGE_DRAG_AMOUNT {
                                    return self.update(ctx, ReaderMsg::NextPage);
                                } else if let Some(section) = self.get_current_section() {
                                    section.transitioning_page(0);
                                    self.drag_distance = 0;
                                }
                            }

                            // Clicked on a[href]
                            DragType::None => {
                                if type_of == DragType::None {
                                    if let Some(dur) = instant {
                                        if dur.num_milliseconds() < 500 {
                                            if let Some(section) = self.get_current_section() {
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

                        ctx.props().event.emit(ReaderEvent::ViewOverlay(event));
                    }
                }
            }

            ReaderMsg::HandleScrollChangePage(type_of) => {
                match type_of {
                    // Scrolling up
                    DragType::Up(_) => {
                        if self.viewing_chapter != 0 {
                            // TODO?: Ensure we've been stopped at the edge for at least 1 second before performing page change steps.
                            // Scrolling is split into 5 sections. You need to scroll up or down at least 3 time to change page to next after timeout.
                            // At 5 we switch automatically. It should also take 5 MAX to fill the current reader window.

                            let height = ctx.props().settings.dimensions.1 as isize / 5;

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
                        if self.viewing_chapter + 1 != self.sections.len() {
                            let height = ctx.props().settings.dimensions.1 as isize / 5;

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
                let height = ctx.props().settings.dimensions.1 as isize / 5;

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

            ReaderMsg::SetPage(new_page) => {
                match self.cached_display {
                    SectionDisplay::Single(_) | SectionDisplay::Double(_) => {
                        return self
                            .set_page(new_page.min(self.page_count(ctx).saturating_sub(1)), ctx);
                    }

                    SectionDisplay::Scroll(_) => {
                        if self.set_section(
                            new_page.min(ctx.props().book.chapter_count.saturating_sub(1)),
                            ctx,
                        ) {
                            self.upload_progress_and_emit(ctx);

                            return true;
                        } else {
                            // We couldn't set the chapter which means we have to load it.
                            // TODO: Should we do anything here? Chapter should be requested and starting to load at this point.
                        }
                    }
                }
            }

            ReaderMsg::NextPage => {
                match self.cached_display {
                    SectionDisplay::Single(_) | SectionDisplay::Double(_) => {
                        if self.current_page_pos() + 1 == self.page_count(ctx) {
                            return false;
                        }

                        self.next_page();
                    }

                    SectionDisplay::Scroll(_) => {
                        if self.viewing_chapter + 1 == self.sections.len() {
                            return false;
                        }

                        self.set_section(self.viewing_chapter + 1, ctx);

                        self.upload_progress_and_emit(ctx);
                    }
                }

                self.drag_distance = 0;
            }

            ReaderMsg::PreviousPage => {
                match self.cached_display {
                    SectionDisplay::Single(_) | SectionDisplay::Double(_) => {
                        if self.current_page_pos() == 0 {
                            return false;
                        }

                        self.previous_page();
                    }

                    SectionDisplay::Scroll(_) => {
                        if self.viewing_chapter == 0 {
                            return false;
                        }

                        self.set_section(self.viewing_chapter - 1, ctx);

                        self.upload_progress_and_emit(ctx);
                    }
                }

                self.drag_distance = 0;
            }

            ReaderMsg::UploadProgress => self.upload_progress_and_emit(ctx),

            // Called after iframe is loaded.
            ReaderMsg::GenerateIFrameLoaded(chapter) => {
                self.sections[chapter.value].convert_to_loaded();

                // Call on_load for the newly loaded frame.
                if let SectionLoadProgress::Loaded(section) = &mut self.sections[chapter.value]
                {
                    section.on_load(
                        &mut self.cached_display,
                        ctx.props().settings.dimensions,
                        &self.handle_js_redirect_clicks,
                        ctx,
                    );
                }

                if self.are_all_sections_generated() {
                    self.on_all_frames_generated(ctx);
                }

                self.update_cached_pages();

                self.use_progression(*ctx.props().progress.lock().unwrap(), ctx);

                // Make sure the previous section is on the last page for better page turning on initial load.
                if let Some(prev_sect) = get_previous_section_mut!(self) {
                    self.cached_display.set_last_page(prev_sect)
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
            ctx.props().settings.dimensions.0,
            ctx.props().settings.dimensions.1
        );

        let progress_percentage = match self.cached_display {
            SectionDisplay::Double(_) | SectionDisplay::Single(_) => format!(
                "width: {}%;",
                (self.current_page_pos() + 1) as f64 / page_count as f64 * 100.0
            ),
            SectionDisplay::Scroll(_) => format!(
                "width: {}%;",
                (self.viewing_chapter + 1) as f64 / section_count as f64 * 100.0
            ),
        };

        let (frame_class, frame_style) = self.get_frame_class_and_style();

        let link = ctx.link().clone();

        html! {
            <div class="reader">
                { self.render_navbar(ctx) }

                <div class="pages" style={ pages_style.clone() }>
                    {
                        if !self.cached_display.is_scroll() {
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
                        ontransitionend={ Callback::from(move|_| link.send_message(ReaderMsg::PageTransitionEnd)) }
                    >
                        {
                            for (0..section_count)
                                .into_iter()
                                .map(|i| {
                                    if let Some(v) = self.sections[i].as_chapter() {
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

                {
                    if ctx.props().settings.show_progress {
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

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let props = ctx.props();

        if self.cached_display != props.settings.display
            || self.cached_dimensions != Some(props.settings.dimensions)
        {
            self.cached_display = props.settings.display.clone();
            self.cached_dimensions = Some(props.settings.dimensions);

            // Refresh all page styles and sizes.
            for prog in &self.sections {
                if let SectionLoadProgress::Loaded(section) = prog {
                    update_iframe_size(Some(props.settings.dimensions), section.get_iframe());

                    self.cached_display.add_to_iframe(section.get_iframe(), ctx);
                }
            }

            self.update_cached_pages();
        }

        match props.settings.type_of {
            PageLoadType::All => {
                let chapter_count = props.book.chapter_count;

                // Reverse iterator since for some reason chapter "generation" works from LIFO
                for chapter in (0..chapter_count).rev() {
                    self.load_section(chapter, ctx);
                }
            }

            PageLoadType::Select => {
                let start_chapter = {
                    if let Some(Progression::Ebook { chapter, .. }) =
                        *props.progress.lock().unwrap()
                    {
                        chapter
                    } else {
                        0
                    }
                };

                // Continue loading chapters
                let start = (start_chapter - 2).max(0) as usize;

                // Reverse iterator since for some reason chapter "generation" works from LIFO
                for chapter in (start..start + 5).rev() {
                    self.load_section(chapter, ctx);
                }
            }
        }

        self.use_progression(*props.progress.lock().unwrap(), ctx);

        true
    }
}

impl Reader {
    fn render_navbar(&self, ctx: &Context<Self>) -> Html {
        let page_count = self.page_count(ctx);
        let section_count = ctx.props().book.chapter_count;

        html! {
            <div class="navbar">
                {
                    match self.cached_display {
                        SectionDisplay::Double(_) | SectionDisplay::Single(_) => html! {
                            <>
                                <a onclick={ ctx.link().callback(|_| ReaderMsg::SetPage(0)) }>{ "First Page" }</a>
                                <a onclick={ ctx.link().callback(|_| ReaderMsg::PreviousPage) }>{ "Previous Page" }</a>
                                <span>{ "Page " } { self.current_page_pos() + 1 } { "/" } { page_count }</span>
                                <a onclick={ ctx.link().callback(|_| ReaderMsg::NextPage) }>{ "Next Page" }</a>
                                <a onclick={ ctx.link().callback(move |_| ReaderMsg::SetPage(page_count - 1)) }>{ "Last Page" }</a>
                            </>
                        },

                        SectionDisplay::Scroll(_) => html! {
                            <>
                                <a onclick={ ctx.link().callback(|_| ReaderMsg::SetPage(0)) }>{ "First Section" }</a>
                                <a onclick={ ctx.link().callback(|_| ReaderMsg::PreviousPage) }>{ "Previous Section" }</a>
                                <span><b>{ "Section " } { self.viewing_chapter + 1 } { "/" } { section_count }</b></span>
                                <a onclick={ ctx.link().callback(|_| ReaderMsg::NextPage) }>{ "Next Section" }</a>
                                <a onclick={ ctx.link().callback(move |_| ReaderMsg::SetPage(section_count - 1)) }>{ "Last Section" }</a>
                            </>
                        }
                    }
                }
            </div>
        }
    }

    fn get_frame_class_and_style(&self) -> (&'static str, String) {
        if self.cached_display.is_scroll() {
            (
                "frames",
                format!(
                    "top: calc(-{}% + {}px); {}",
                    self.viewing_chapter * 100,
                    self.drag_distance,
                    Some("transition: top 0.5s ease 0s;").unwrap_or_default()
                ),
            )
        } else {
            let mut transition = Some("transition: left 0.5s ease 0s;");

            // Prevent empty pages when on the first or last page of a section.
            let amount = if self.drag_distance.is_positive() {
                if self
                    .get_current_section()
                    .map(|v| v.viewing_page() == 0)
                    .unwrap_or_default()
                {
                    transition = None;
                    self.drag_distance
                } else {
                    0
                }
            } else if self.drag_distance.is_negative() {
                if self
                    .get_current_section()
                    .map(|v| v.viewing_page() == v.page_count().saturating_sub(1))
                    .unwrap_or_default()
                {
                    transition = None;
                    self.drag_distance
                } else {
                    0
                }
            } else {
                0
            };

            (
                "frames horizontal",
                format!(
                    "left: calc(-{}% + {}px); {}",
                    self.viewing_chapter * 100,
                    amount,
                    transition.unwrap_or_default()
                ),
            )
        }
    }

    fn use_progression(&mut self, prog: Option<Progression>, ctx: &Context<Self>) {
        log::info!("{:?}", prog);

        if let Some(prog) = prog {
            match prog {
                Progression::Ebook {
                    chapter, char_pos, ..
                } if self.viewing_chapter != chapter as usize => {
                    log::debug!("use_progression - set section: {chapter}");

                    // TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
                    self.set_section(chapter as usize, ctx);

                    if char_pos != -1 {
                        if let SectionLoadProgress::Loaded(section) =
                            &mut self.sections[chapter as usize]
                        {
                            if self.cached_display.is_scroll() {
                                if let Some(_element) = js_get_element_from_byte_position(
                                    section.get_iframe(),
                                    char_pos as usize,
                                ) {
                                    // TODO: Not scrolling properly. Is it somehow scrolling the div@frames html element?
                                    // element.scroll_into_view();
                                }
                            } else {
                                let page = js_get_page_from_byte_position(
                                    section.get_iframe(),
                                    char_pos as usize,
                                );

                                log::debug!("use_progression - set page: {:?}", page);

                                if let Some(page) = page {
                                    self.cached_display.set_page(page, section);
                                }
                            }
                        }
                    }
                }

                _ => (),
            }
        }
    }

    fn are_all_sections_generated(&self) -> bool {
        self.sections.iter().all(|v| v.is_loaded())
    }

    fn update_cached_pages(&mut self) {
        let mut total_page_pos = 0;

        // TODO: Verify if needed. Or can we do values_mut() we need to have it in asc order
        for chapter in 0..self.sections.len() {
            if let SectionLoadProgress::Loaded(ele) = &mut self.sections[chapter] {
                let page_count = get_iframe_page_count(ele.get_iframe()).max(1);

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
        log::info!("All Frames Generated");
        // Double check page counts before proceeding.
        self.update_cached_pages();

        // TODO: Move to Msg::GenerateIFrameLoaded so it's only in a single place.
        self.use_progression(*ctx.props().progress.lock().unwrap(), ctx);
    }

    fn next_page(&mut self) -> bool {
        let viewing_chapter = self.viewing_chapter;
        let section_count = self.sections.len();

        if let Some(curr_sect) = get_current_section_mut!(self) {
            if self.cached_display.next_page(curr_sect) {
                return true;
            } else {
                curr_sect.transitioning_page(0);
            }

            if viewing_chapter + 1 != section_count {
                self.cached_display.on_stop_viewing(curr_sect);

                self.viewing_chapter += 1;

                // Make sure the next sections viewing page is zero.
                if let Some(next_sect) = get_current_section_mut!(self) {
                    self.cached_display.set_page(0, next_sect);
                    self.cached_display.on_start_viewing(next_sect);
                }

                return true;
            }
        }

        false
    }

    fn previous_page(&mut self) -> bool {
        if let Some(curr_sect) = get_current_section_mut!(self) {
            if self.cached_display.previous_page(curr_sect) {
                return true;
            } else {
                curr_sect.transitioning_page(0);
            }

            if self.viewing_chapter != 0 {
                self.cached_display.on_stop_viewing(curr_sect);

                self.viewing_chapter -= 1;

                // Make sure the next sections viewing page is maxed.
                if let Some(next_sect) = get_current_section_mut!(self) {
                    self.cached_display.set_last_page(next_sect);
                    self.cached_display.on_start_viewing(next_sect);
                }

                return true;
            }
        }

        false
    }

    /// Expensive. Iterates through previous sections.
    fn set_page(&mut self, new_total_page: usize, ctx: &Context<Self>) -> bool {
        for chap in 0..ctx.props().book.chapter_count {
            if let SectionLoadProgress::Loaded(section) = &mut self.sections[chap] {
                // This should only happen if the page isn't loaded for some reason.
                if new_total_page < section.gpi {
                    break;
                }

                let local_page = new_total_page - section.gpi;

                if local_page < section.page_count() {
                    self.viewing_chapter = section.chapter();

                    self.cached_display.set_page(local_page, section);

                    return true;
                }
            }
        }

        false
    }

    fn set_section(&mut self, next_section: usize, ctx: &Context<Self>) -> bool {
        if self.sections[next_section].is_waiting() {
            log::info!("Next Section is not loaded - {}", next_section + 1);

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

        if let Some(section) = self.get_current_section() {
            self.cached_display.on_stop_viewing(section);
        }

        if let SectionLoadProgress::Loaded(section) = &mut self.sections[next_section] {
            self.viewing_chapter = next_section;

            self.cached_display.set_page(0, section);
            self.cached_display.on_start_viewing(section);

            true
        } else {
            false
        }
    }

    fn after_page_change(&mut self) {
        if let Some(section) = self.get_current_section() {
            self.visible_redirect_rects =
                js_get_visible_links(section.get_iframe(), self.cached_display.is_scroll());
        }
    }

    /// Expensive. Iterates through sections backwards from last -> first.
    fn page_count(&self, ctx: &Context<Self>) -> usize {
        let section_count = ctx.props().book.chapter_count;

        for index in 1..=section_count {
            if let Some(pos) = self
                .sections
                .get(section_count - index)
                .and_then(|s| Some(s.as_loaded()?.get_page_count_until()))
            {
                return pos;
            }
        }

        0
    }

    fn current_page_pos(&self) -> usize {
        self.get_current_section()
            .map(|s| s.gpi + s.viewing_page())
            .unwrap_or_default()
    }

    fn get_current_section(&self) -> Option<&SectionContents> {
        self.sections
            .get(self.viewing_chapter)
            .and_then(|v| v.as_chapter())
    }

    fn upload_progress_and_emit(&self, ctx: &Context<Self>) {
        if let Some(chap) = self.get_current_section() {
            self.upload_progress(chap.get_iframe(), ctx);
        }
    }

    fn upload_progress(&self, iframe: &HtmlIFrameElement, ctx: &Context<Self>) {
        let (chapter, page, char_pos, book_id) = (
            self.viewing_chapter,
            self.get_current_section()
                .map(|v| v.viewing_page())
                .unwrap_or_default() as i64,
            js_get_current_byte_pos(iframe)
                .map(|v| v as i64)
                .unwrap_or(-1),
            ctx.props().book.id,
        );

        let last_page = self.page_count(ctx).saturating_sub(1);

        let stored_prog = Rc::clone(&ctx.props().progress);

        let req = match self.page_count(ctx) {
            0 if chapter == 0 => {
                *stored_prog.lock().unwrap() = None;

                None
            }

            // TODO: Figure out what the last page of each book actually is.
            v if v as usize == last_page && chapter == self.sections.len().saturating_sub(1) => {
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

                *stored_prog.lock().unwrap() = value;

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

    fn refresh_section(&mut self, chapter: usize, ctx: &Context<Self>) {
        let chaps = ctx.props().chapters.lock().unwrap();

        if chaps.chapters.len() <= chapter {
            return;
        }

        let chap = chaps.chapters[chapter].clone();

        let sec = &mut self.sections[chap.value];

        if let SectionLoadProgress::Loaded(sec) = sec {
            sec.get_iframe().remove();
        }

        *sec = SectionLoadProgress::Loading(generate_pages(
            Some(ctx.props().settings.dimensions),
            ctx.props().book.id,
            chap,
            ctx.link().clone(),
        ));
    }

    fn load_section(&mut self, chapter: usize, ctx: &Context<Self>) {
        let chaps = ctx.props().chapters.lock().unwrap();

        if chaps.chapters.len() <= chapter {
            return;
        }

        let chap = chaps.chapters[chapter].clone();

        let sec = &mut self.sections[chap.value];

        if sec.is_waiting() {
            log::info!("Generating Chapter {}", chap.value + 1);

            *sec = SectionLoadProgress::Loading(generate_pages(
                Some(ctx.props().settings.dimensions),
                ctx.props().book.id,
                chap,
                ctx.link().clone(),
            ));
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

fn generate_pages(
    book_dimensions: Option<(i32, i32)>,
    book_id: FileId,
    chapter: Chapter,
    scope: Scope<Reader>,
) -> SectionContents {
    let iframe = create_iframe();

    iframe.set_attribute("fetchPriority", "low").unwrap();

    iframe
        .set_attribute(
            "src",
            &request::compile_book_resource_path(
                book_id,
                &chapter.file_path,
                api::LoadResourceQuery {
                    configure_pages: true,
                },
            ),
        )
        .unwrap();

    update_iframe_size(book_dimensions, &iframe);

    let new_frame = iframe.clone();

    let chap_value = chapter.value;

    let f = Closure::wrap(Box::new(move || {
        let chapter = chapter.clone();

        scope.send_message(ReaderMsg::GenerateIFrameLoaded(chapter));
    }) as Box<dyn FnMut()>);

    new_frame.set_onload(Some(f.as_ref().unchecked_ref()));

    SectionContents::new(chap_value, new_frame, f)
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
        .set_property("width", &format!("{}px", width))
        .unwrap();
    iframe
        .style()
        .set_property("height", &format!("{}px", height))
        .unwrap();
}
