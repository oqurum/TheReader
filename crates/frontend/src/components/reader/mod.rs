use std::{collections::HashMap, rc::Rc, sync::Mutex, path::PathBuf, cell::Cell};

use chrono::Utc;
use common_local::{MediaItem, Progression, Chapter, api, FileId};
use gloo_timers::callback::Timeout;
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}, UnwrapThrowExt};
use web_sys::{HtmlIFrameElement, HtmlElement, Document};
use yew::{prelude::*, html::Scope};

use crate::request;


mod layout;
mod view_overlay;

pub use self::layout::SectionDisplay;
pub use self::view_overlay::{ViewOverlay, OverlayEvent, DragType};


const PAGE_CHANGE_DRAG_AMOUNT: usize = 200;


#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
    // TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
    fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;

    fn js_get_current_byte_pos(iframe: &HtmlIFrameElement) -> Option<usize>;
    fn js_get_page_from_byte_position(iframe: &HtmlIFrameElement, position: usize) -> Option<usize>;
    fn js_get_element_from_byte_position(iframe: &HtmlIFrameElement, position: usize) -> Option<HtmlElement>;

    fn js_update_iframe_after_load(iframe: &HtmlIFrameElement, chapter: usize, handle_js_redirect_clicks: &Closure<dyn FnMut(usize, String)>);
    fn js_set_page_display_style(iframe: &HtmlIFrameElement, display: u8);
}



#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PageLoadType {
    All,
    #[default]
    Select,
}


#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReaderSettings {
    pub load_speed: usize,
    pub type_of: PageLoadType,

    pub is_fullscreen: bool,
    pub display: SectionDisplay,
    pub show_progress: bool,

    pub dimensions: (i32, i32),
}



#[derive(Clone, Copy, PartialEq, Eq)]
struct CachedPage {
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
    LoadChapters,
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


pub enum Msg {
    GenerateIFrameLoaded(GenerateChapter),

    // Event
    HandleJsRedirect(usize, String, Option<String>),

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
    cached_display: SectionDisplay,
    cached_dimensions: Option<(i32, i32)>,

    // All the sections the books has and the current cached info
    sections: HashMap<usize, BookSection>,

    /// The Chapter we're in
    viewing_chapter: usize,

    handle_js_redirect_clicks: Closure<dyn FnMut(usize, String)>,

    drag_distance: isize,

    scroll_change_page_timeout: Option<Timeout>,
}

impl Component for Reader {
    type Message = Msg;
    type Properties = Property;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let handle_js_redirect_clicks = Closure::wrap(Box::new(move |chapter: usize, path: String| {
            let (file_path, id_value) = path.split_once('#')
                .map(|(a, b)| (a.to_string(), Some(b.to_string())))
                .unwrap_or((path, None));

            link.send_message(Msg::HandleJsRedirect(chapter, file_path, id_value));
        }) as Box<dyn FnMut(usize, String)>);

        Self {
            cached_display: ctx.props().settings.display,
            cached_dimensions: None,
            sections: {
                let mut map = HashMap::new();

                for i in 0..ctx.props().book.chapter_count {
                    map.insert(i, BookSection::Waiting);
                }

                map
            },

            viewing_chapter: 0,
            drag_distance: 0,

            scroll_change_page_timeout: None,

            handle_js_redirect_clicks,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => return false,

            Msg::HandleJsRedirect(_chapter, file_path, _id_name) => {
                let file_path = PathBuf::from(file_path);

                let chaps = ctx.props().chapters.lock().unwrap();

                // TODO: Ensure we handle any paths which go to a parent directory. eg: "../file.html"
                // let mut path = chaps.chapters.iter().find(|v| v.value == chapter).unwrap().file_path.clone();
                // path.pop();

                if let Some(chap) = chaps.chapters.iter().find(|v| v.file_path.ends_with(&file_path)) {
                    self.set_section(chap.value, ctx);
                    // TODO: Handle id_name
                }
            }

            Msg::HandleViewOverlay(event) => {
                match event.type_of {
                    DragType::Up(_distance) => (),
                    DragType::Down(_distance) => (),

                    // Previous Page
                    DragType::Right(distance) => {
                        if event.dragging {
                            self.drag_distance = distance as isize;

                            if let Some(section) = self.get_current_section() {
                                section.transitioning_page(self.drag_distance);
                            }
                        } else if distance > PAGE_CHANGE_DRAG_AMOUNT {
                            return self.update(ctx, Msg::PreviousPage);
                        } else if let Some(section) = self.get_current_section() {
                            section.transitioning_page(0);
                            self.drag_distance = 0;
                        }
                    }

                    // Next Page
                    DragType::Left(distance) => {
                        if event.dragging {
                            self.drag_distance = -(distance as isize);

                            if let Some(section) = self.get_current_section() {
                                section.transitioning_page(self.drag_distance);
                            }
                        } else if distance > PAGE_CHANGE_DRAG_AMOUNT {
                            return self.update(ctx, Msg::NextPage);
                        } else if let Some(section) = self.get_current_section() {
                            section.transitioning_page(0);
                            self.drag_distance = 0;
                        }
                    }

                    DragType::None => (),
                }

                ctx.props().event.emit(ReaderEvent::ViewOverlay(event));
            }

            Msg::HandleScrollChangePage(type_of) => {
                match type_of {
                    // Scrolling up
                    DragType::Up(_) => if self.viewing_chapter != 0 {
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
                            self.scroll_change_page_timeout = Some(Timeout::new(1_000, move || {
                                link.send_message(Msg::UpdateDragDistance);
                            }));
                        }
                    }

                    // Scrolling down
                    DragType::Down(_) => if self.viewing_chapter + 1 != self.sections.len() {
                        let height = ctx.props().settings.dimensions.1 as isize / 5;

                        self.drag_distance -= height;

                        if self.drag_distance.abs() / height == 5 {
                            self.drag_distance = 0;
                            self.previous_page();
                        } else {
                            // After 500 ms of no scroll activity reset position ( self.drag_distance ?? ) to ZERO.
                            let link = ctx.link().clone();
                            self.scroll_change_page_timeout = Some(Timeout::new(1_000, move || {
                                link.send_message(Msg::UpdateDragDistance);
                            }));
                        }
                    }

                    _ => unreachable!(),
                }
            }

            Msg::UpdateDragDistance => {
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


            Msg::SetPage(new_page) => {
                match self.cached_display {
                    SectionDisplay::Single | SectionDisplay::Double => {
                        return self.set_page(new_page.min(self.page_count(ctx).saturating_sub(1)), ctx);
                    }

                    SectionDisplay::Scroll => {
                        if self.set_section(new_page.min(ctx.props().book.chapter_count.saturating_sub(1)), ctx) {
                            self.upload_progress_and_emit(ctx);

                            return true;
                        } else {
                            // We couldn't set the chapter which means we have to load it.
                            // TODO: Should we do anything here? Chapter should be requested and starting to load at this point.
                        }
                    }
                }
            }

            Msg::NextPage => {
                match self.cached_display {
                    SectionDisplay::Single | SectionDisplay::Double => {
                        if self.current_page_pos() + 1 == self.page_count(ctx) {
                            return false;
                        }

                        self.next_page();
                    }

                    SectionDisplay::Scroll => {
                        if self.viewing_chapter + 1 == self.sections.len() {
                            return false;
                        }

                        self.set_section(self.viewing_chapter + 1, ctx);

                        self.upload_progress_and_emit(ctx);
                    }
                }

                self.drag_distance = 0;
            }

            Msg::PreviousPage => {
                match self.cached_display {
                    SectionDisplay::Single | SectionDisplay::Double => {
                        if self.current_page_pos() == 0 {
                            return false;
                        }

                        self.previous_page();
                    }

                    SectionDisplay::Scroll => {
                        if self.viewing_chapter == 0 {
                            return false;
                        }

                        self.set_section(self.viewing_chapter - 1, ctx);

                        self.upload_progress_and_emit(ctx);
                    }
                }

                self.drag_distance = 0;
            }

            Msg::UploadProgress => self.upload_progress_and_emit(ctx),

            // Called after iframe is loaded.
            Msg::GenerateIFrameLoaded(page) => {
                js_update_iframe_after_load(&page.iframe, page.chapter.value, &self.handle_js_redirect_clicks);

                { // Page changes use a transition. After the transition ends we'll upload the progress. Fixes the issue of js_get_current_by_pos being incorrect.
                    let link = ctx.link().clone();
                    let f = Closure::wrap(Box::new(move || link.send_message(Msg::UploadProgress)) as Box<dyn FnMut()>);

                    let body = page.iframe.content_document().unwrap().body().unwrap();
                    body.set_ontransitionend(Some(f.as_ref().unchecked_ref()));

                    f.forget();
                }

                { // Scroll Display - Used to handle section changing with the scroll wheel.
                    let link = ctx.link().clone();
                    let f = Closure::wrap(Box::new(move |e: WheelEvent| {
                        let el: Document = e.current_target().unwrap_throw().unchecked_into();
                        let scrolling_element = el.scrolling_element().unwrap_throw();

                        let delta = e.delta_y();
                        let is_scrolling_down = delta.is_sign_positive();

                        if !is_scrolling_down && scrolling_element.scroll_top() == 0 {
                            // At the start
                            link.send_message(Msg::HandleScrollChangePage(DragType::Up(delta.abs() as usize)));
                        } else if is_scrolling_down && scrolling_element.scroll_top() + scrolling_element.client_height() >= scrolling_element.scroll_height() {
                            // At the end
                            link.send_message(Msg::HandleScrollChangePage(DragType::Down(delta.abs() as usize)));
                        }
                    }) as Box<dyn FnMut(WheelEvent)>);

                    // TODO: Having the event here prevents simple section changes
                    let body = page.iframe.content_document().unwrap();
                    body.set_onwheel(Some(f.as_ref().unchecked_ref()));

                    f.forget();
                }

                { // Scroll Display - On click
                    let link = ctx.link().clone();
                    let press_duration = Rc::new(Cell::new(Utc::now()));

                    let press_duration2 = press_duration.clone();
                    let f_md = Closure::wrap(Box::new(move || {
                        press_duration2.set(Utc::now());
                    }) as Box<dyn FnMut()>);

                    let f_mu = Closure::wrap(Box::new(move || {
                        let duration = Utc::now().signed_duration_since(press_duration.get());

                        link.send_message(Msg::HandleViewOverlay(OverlayEvent {
                            type_of: DragType::None,
                            dragging: false,
                            instant: Some(duration),
                        }))
                    }) as Box<dyn FnMut()>);

                    let body = page.iframe.content_document().unwrap_throw();
                    body.set_onmousedown(Some(f_md.as_ref().unchecked_ref()));
                    body.set_onmouseup(Some(f_mu.as_ref().unchecked_ref()));

                    f_md.forget();
                    f_mu.forget();
                }

                {
                    let gen = self.sections.remove(&page.chapter.value).unwrap();
                    self.sections.insert(page.chapter.value, gen.convert_to_loaded());
                }

                // Update newly iframe with styling and size.
                if let Some(BookSection::Loaded(sec)) = self.sections.get(&page.chapter.value) {
                    sec.on_stop_viewing(self.cached_display);
                    js_set_page_display_style(&sec.iframe, self.cached_display.into());
                    update_iframe_size(Some(ctx.props().settings.dimensions), &sec.iframe);
                }


                let loading_count = self.sections.values().filter(|v| v.is_loading()).count();

                if self.are_all_sections_generated() {
                    self.on_all_frames_generated(ctx);
                }

                self.update_cached_pages();

                self.use_progression(*ctx.props().progress.lock().unwrap(), ctx);

                let display = self.cached_display;

                // Make sure the previous section is on the last page for better page turning on initial load.
                if let Some(prev_sect) = self.get_previous_section_mut() {
                    prev_sect.set_last_page(display);
                }

                if loading_count == 0 {
                    ctx.props().event.emit(ReaderEvent::LoadChapters);
                }
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let page_count = self.page_count(ctx);
        let section_count = ctx.props().book.chapter_count;

        let pages_style = format!("width: {}px; height: {}px;", ctx.props().settings.dimensions.0, ctx.props().settings.dimensions.1);

        let progress_percentage = match self.cached_display {
            SectionDisplay::Double | SectionDisplay::Single => format!("width: {}%;", (self.current_page_pos() + 1) as f64 / page_count as f64 * 100.0),
            SectionDisplay::Scroll => format!("width: {}%;", (self.viewing_chapter + 1) as f64 / section_count as f64 * 100.0),
        };


        let (frame_class, frame_style) = if self.cached_display == SectionDisplay::Scroll {
            (
                "frames",
                format!(
                    "top: calc(-{}% + {}px); {}",
                    self.viewing_chapter * 100,
                    self.drag_distance,
                    Some("transition: top 0.5s ease 0s;").unwrap_or_default()
                )
            )
        } else {
            let mut transition = Some("transition: left 0.5s ease 0s;");

            // Prevent empty pages when on the first or last page of a section.
            let amount = if self.drag_distance.is_positive() {
                if self.get_current_section().map(|v| v.viewing_page == 0).unwrap_or_default() {
                    transition = None;
                    self.drag_distance
                } else {
                    0
                }
            } else if self.drag_distance.is_negative() {
                if self.get_current_section().map(|v| v.viewing_page == v.page_count().saturating_sub(1)).unwrap_or_default() {
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
                format!("left: calc(-{}% + {}px); {}", self.viewing_chapter * 100, amount, transition.unwrap_or_default())
            )
        };

        let link = ctx.link().clone();

        html! {
            <div class="reader">
                <div class="navbar">
                    {
                        match self.cached_display {
                            SectionDisplay::Double | SectionDisplay::Single => html! {
                                <>
                                    <a onclick={ ctx.link().callback(|_| Msg::SetPage(0)) }>{ "First Page" }</a>
                                    <a onclick={ ctx.link().callback(|_| Msg::PreviousPage) }>{ "Previous Page" }</a>
                                    <span>{ "Page " } { self.current_page_pos() + 1 } { "/" } { page_count }</span>
                                    <a onclick={ ctx.link().callback(|_| Msg::NextPage) }>{ "Next Page" }</a>
                                    <a onclick={ ctx.link().callback(move |_| Msg::SetPage(page_count - 1)) }>{ "Last Page" }</a>
                                </>
                            },

                            SectionDisplay::Scroll => html! {
                                <>
                                    <a onclick={ ctx.link().callback(|_| Msg::SetPage(0)) }>{ "First Section" }</a>
                                    <a onclick={ ctx.link().callback(|_| Msg::PreviousPage) }>{ "Previous Section" }</a>
                                    <span><b>{ "Section " } { self.viewing_chapter + 1 } { "/" } { section_count }</b></span>
                                    <a onclick={ ctx.link().callback(|_| Msg::NextPage) }>{ "Next Section" }</a>
                                    <a onclick={ ctx.link().callback(move |_| Msg::SetPage(section_count - 1)) }>{ "Last Section" }</a>
                                </>
                            }
                        }
                    }
                </div>

                <div class="pages" style={ pages_style.clone() }>
                    {
                        if self.cached_display != SectionDisplay::Scroll {
                            html! {
                                <ViewOverlay event={ ctx.link().callback(Msg::HandleViewOverlay) } />
                            }
                        } else {
                            html! {}
                        }
                    }
                    <div
                        class={ frame_class }
                        style={ frame_style }
                        // Frame changes use a transition. After the transition ends we'll upload the progress.
                        ontransitionend={ Callback::from(move|_| link.send_message(Msg::UploadProgress)) }
                    >
                        {
                            for (0..section_count)
                                .into_iter()
                                .map(|i| {
                                    if let Some(v) = self.sections.get(&i).unwrap().as_chapter() {
                                        Html::VRef(v.iframe.clone().into())
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

        if self.cached_display != props.settings.display || self.cached_dimensions != Some(props.settings.dimensions) {
            self.cached_display = props.settings.display;
            self.cached_dimensions = Some(props.settings.dimensions);

            // Refresh all page styles and sizes.
            for chap in self.sections.values() {
                if let BookSection::Loaded(chap) = chap {
                    js_set_page_display_style(&chap.iframe, self.cached_display.into());
                    update_iframe_size(Some(props.settings.dimensions), &chap.iframe);
                }
            }

            self.update_cached_pages();
        }

        // TODO: Move to Msg::GenerateIFrameLoaded so it's only in a single place.
        self.use_progression(*props.progress.lock().unwrap(), ctx);

        // Continue loading chapters
        let chaps = props.chapters.lock().unwrap();

        // Reverse iterator since for some reason chapter "generation" works from LIFO
        for chap in chaps.chapters.iter().rev() {
            if let Some(sec) = self.sections.get_mut(&chap.value) {
                if sec.is_waiting() {
                    log::info!("Generating Chapter {}", chap.value + 1);

                    *sec = BookSection::Loading(generate_pages(
                        Some(props.settings.dimensions),
                        props.book.id,
                        chap.clone(),
                        ctx.link().clone()
                    ));
                }
            }
        }

        true
    }
}

impl Reader {
    fn use_progression(&mut self, prog: Option<Progression>, ctx: &Context<Self>) {
        if let Some(prog) = prog {
            match prog {
                Progression::Ebook { chapter, char_pos, .. } if self.viewing_chapter == 0 => {
                    if self.sections.contains_key(&(chapter as usize)) {
                        // TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
                        self.set_section(chapter as usize, ctx);

                        if char_pos != -1 {
                            let book_section = self.sections.get_mut(&(chapter as usize)).unwrap();

                            if let BookSection::Loaded(section) = book_section {
                                if self.cached_display == SectionDisplay::Scroll {
                                    if let Some(_element) = js_get_element_from_byte_position(&section.iframe, char_pos as usize) {
                                        // TODO: Not scrolling properly. Is it somehow scrolling the div@frames html element?
                                        // element.scroll_into_view();
                                    }
                                } else {
                                    let page = js_get_page_from_byte_position(&section.iframe, char_pos as usize);

                                    if let Some(page) = page {
                                        section.set_page(page, self.cached_display);
                                    }
                                }
                            }

                        }
                    }
                }

                _ => ()
            }
        }
    }

    fn are_all_sections_generated(&self) -> bool {
        self.sections.values().all(|v| v.is_loaded())
    }

    fn update_cached_pages(&mut self) {
        let mut total_page_pos = 0;

        // TODO: Verify if needed. Or can we do values_mut() we need to have it in asc order
        for chap in 0..self.sections.len() {
            if let Some(BookSection::Loaded(ele)) = self.sections.get_mut(&chap) {
                let page_count = get_iframe_page_count(&ele.iframe).max(1);

                ele.gpi = total_page_pos;

                total_page_pos += page_count;

                ele.cached_pages.clear();

                for local_page in 0..page_count {
                    ele.cached_pages.push(CachedPage {
                        chapter: ele.chapter,
                        chapter_local_page: local_page
                    });
                }
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
        let display = self.cached_display;
        let viewing_chapter = self.viewing_chapter;
        let section_count = self.sections.len();

        if let Some(curr_sect) = self.get_current_section_mut() {
            if curr_sect.next_page(display) {
                return true;
            } else {
                curr_sect.transitioning_page(0);
            }

            if viewing_chapter + 1 != section_count {
                curr_sect.on_stop_viewing(display);

                self.viewing_chapter += 1;

                // Make sure the next sections viewing page is zero.
                if let Some(next_sect) = self.get_current_section_mut() {
                    next_sect.set_page(0, display);
                    next_sect.on_start_viewing(display);
                }

                return true;
            }
        }

        false
    }

    fn previous_page(&mut self) -> bool {
        let display = self.cached_display;
        let viewing_chapter = self.viewing_chapter;

        if let Some(curr_sect) = self.get_current_section_mut() {
            if curr_sect.previous_page(display) {
                return true;
            } else {
                curr_sect.transitioning_page(0);
            }

            if viewing_chapter != 0 {
                curr_sect.on_stop_viewing(display);

                self.viewing_chapter -= 1;

                // Make sure the next sections viewing page is maxed.
                if let Some(next_sect) = self.get_current_section_mut() {
                    next_sect.set_last_page(display);
                    next_sect.on_start_viewing(display);
                }

                return true;
            }
        }

        false
    }

    /// Expensive. Iterates through previous sections.
    fn set_page(&mut self, new_total_page: usize, ctx: &Context<Self>) -> bool {
        for chap in 0..ctx.props().book.chapter_count {
            if let Some(BookSection::Loaded(section)) = self.sections.get_mut(&chap) {
                // This should only happen if the page isn't loaded for some reason.
                if new_total_page < section.gpi {
                    break;
                }

                let local_page = new_total_page - section.gpi;

                if local_page < section.page_count() {
                    self.viewing_chapter = section.chapter;

                    section.set_page(local_page, self.cached_display);

                    return true;
                }
            }
        }

        false
    }

    fn set_section(&mut self, next_section: usize, _ctx: &Context<Self>) -> bool {
        if self.sections.contains_key(&next_section) {
            if let Some(section) = self.get_current_section() {
                section.on_stop_viewing(self.cached_display);
            }
        }

        if let Some(BookSection::Loaded(section)) = self.sections.get_mut(&next_section) {
            self.viewing_chapter = next_section;
            section.viewing_page = 0;
            section.on_start_viewing(self.cached_display);

            true
        } else {
            false
        }
    }


    /// Expensive. Iterates through sections backwards from last -> first.
    fn page_count(&self, ctx: &Context<Self>) -> usize {
        let section_count = ctx.props().book.chapter_count;

        for index in 1..=section_count {
            if let Some(pos) = self.sections.get(&(section_count - index))
                .and_then(|s| Some(s.as_loaded()?.get_page_count_until()))
            {
                return pos;
            }
        }


        0
    }

    fn current_page_pos(&self) -> usize {
        self.get_current_section()
            .map(|s| s.gpi + s.viewing_page)
            .unwrap_or_default()
    }

    fn get_previous_section_mut(&mut self) -> Option<&mut ChapterContents> {
        if let Some(chapter) = self.viewing_chapter.checked_sub(1) {
            self.sections.get_mut(&chapter).and_then(|v| v.as_chapter_mut())
        } else {
            None
        }
    }

    fn get_current_section(&self) -> Option<&ChapterContents> {
        self.sections.get(&self.viewing_chapter).and_then(|v| v.as_chapter())
    }

    fn get_current_section_mut(&mut self) -> Option<&mut ChapterContents> {
        self.sections.get_mut(&self.viewing_chapter).and_then(|v| v.as_chapter_mut())
    }


    fn upload_progress_and_emit(&self, ctx: &Context<Self>) {
        if let Some(chap) = self.get_current_section() {
            self.upload_progress(&chap.iframe, ctx);

            ctx.props().event.emit(ReaderEvent::LoadChapters);
        }
    }

    fn upload_progress(&self, iframe: &HtmlIFrameElement, ctx: &Context<Self>) {
        let (chapter, page, char_pos, book_id) = (
            self.viewing_chapter,
            self.get_current_section().map(|v| v.viewing_page).unwrap_or_default() as i64,
            js_get_current_byte_pos(iframe).map(|v| v as i64).unwrap_or(-1),
            ctx.props().book.id
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
                    page
                });

                *stored_prog.lock().unwrap() = value;

                value
            }
        };

        ctx.link()
        .send_future(async move {
            match req {
                Some(req) => request::update_book_progress(book_id, &req).await,
                None => request::remove_book_progress(book_id).await,
            };

            Msg::Ignore
        });
    }
}



fn create_iframe() -> HtmlIFrameElement {
    gloo_utils::document()
        .create_element("iframe")
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn generate_pages(book_dimensions: Option<(i32, i32)>, book_id: FileId, chapter: Chapter, scope: Scope<Reader>) -> ChapterContents {
    let iframe = create_iframe();

    iframe.set_attribute("fetchPriority", "low").unwrap();

    iframe.set_attribute(
        "src",
        &request::compile_book_resource_path(
            book_id,
            &chapter.file_path,
            api::LoadResourceQuery { configure_pages: true }
        )
    ).unwrap();

    update_iframe_size(book_dimensions, &iframe);

    let new_frame = iframe.clone();

    let chap_value = chapter.value;

    let f = Closure::wrap(Box::new(move || {
        let chapter = chapter.clone();

        scope.send_message(Msg::GenerateIFrameLoaded(GenerateChapter {
            iframe: iframe.clone(),
            chapter
        }));
    }) as Box<dyn FnMut()>);

    new_frame.set_onload(Some(f.as_ref().unchecked_ref()));

    ChapterContents {
        cached_pages: Vec::new(),
        chapter: chap_value,
        iframe: new_frame,
        on_load: f,
        gpi: 0,
        viewing_page: 0,
    }
}

fn update_iframe_size(book_dimensions: Option<(i32, i32)>, iframe: &HtmlIFrameElement) {
    let (width, height) = match book_dimensions { // TODO: Use Option.unzip once stable.
        Some((a, b)) => (a, b),
        None => (gloo_utils::body().client_width().max(0), gloo_utils::body().client_height().max(0)),
    };

    iframe.style().set_property("width", &format!("{}px", width)).unwrap();
    iframe.style().set_property("height", &format!("{}px", height)).unwrap();
}

pub struct GenerateChapter {
    iframe: HtmlIFrameElement,
    chapter: Chapter,
}


pub enum BookSection {
    Waiting,
    Loading(ChapterContents),
    Loaded(ChapterContents)
}

impl BookSection {
    pub fn is_waiting(&self) -> bool {
        matches!(self, Self::Waiting)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading(_))
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    pub fn convert_to_loaded(self) -> Self {
        if let Self::Loading(v) = self {
            Self::Loaded(v)
        } else {
            panic!("unable to convert")
        }
    }

    pub fn as_loaded(&self) -> Option<&ChapterContents> {
        match self {
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_chapter(&self) -> Option<&ChapterContents> {
        match self {
            Self::Loading(v) |
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_chapter_mut(&mut self) -> Option<&mut ChapterContents> {
        match self {
            Self::Loading(v) |
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }
}


pub struct ChapterContents {
    #[allow(dead_code)]
    on_load: Closure<dyn FnMut()>,

    cached_pages: Vec<CachedPage>,

    pub iframe: HtmlIFrameElement,
    pub chapter: usize,

    /// Global Page Index
    pub gpi: usize,

    pub viewing_page: usize,
}

impl ChapterContents {
    fn page_count(&self) -> usize {
        self.cached_pages.len()
    }

    fn get_page_count_until(&self) -> usize {
        self.gpi + self.page_count()
    }

    pub fn transitioning_page(&self, amount: isize) {
        let body = self.iframe.content_document().unwrap().body().unwrap();

        // Prevent empty pages when on the first or last page of a section.
        let amount = if (amount.is_positive() && self.viewing_page == 0) ||
            (amount.is_negative() && self.viewing_page == self.page_count().saturating_sub(1))
        {
            0
        } else {
            amount
        };

        if amount == 0 {
            body.style().set_property("transition", "left 0.5s ease 0s").unwrap();
        } else {
            body.style().remove_property("transition").unwrap();
        }

        body.style().set_property(
            "left",
            &format!(
                "calc(-{}% - {}px)",
                100 * self.viewing_page,
                self.viewing_page as isize * 10 - amount
            )
        ).unwrap();
    }

    pub fn set_last_page(&mut self, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.scroll_with_x_and_y(0.0, el.scroll_height() as f64);
        } else {
            self.set_page(self.page_count().saturating_sub(1), display);
        }
    }

    pub fn set_page(&mut self, page_number: usize, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.scroll_with_x_and_y(0.0, 0.0);
        } else {
            self.viewing_page = page_number;

            let body = self.iframe.content_document().unwrap().body().unwrap();
            body.style().set_property("transition", "left 0.5s ease 0s").unwrap();
            body.style().set_property("left", &format!("calc(-{}% - {}px)", 100 * self.viewing_page, self.viewing_page * 10)).unwrap();
        }

        // TODO: Update Scroll position.
        // TODO: Utilize viewing_page vertically.
    }

    pub fn next_page(&mut self, display: SectionDisplay) -> bool {
        if self.viewing_page + 1 < self.page_count() {
            self.set_page(self.viewing_page + 1, display);

            true
        } else {
            false
        }
    }

    pub fn previous_page(&mut self, display: SectionDisplay) -> bool {
        if self.viewing_page != 0 {
            self.set_page(self.viewing_page - 1, display);

            true
        } else {
            false
        }
    }

    pub fn on_stop_viewing(&self, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.style().set_property("overflow", "hidden").unwrap_throw();
        }
    }

    pub fn on_start_viewing(&self, display: SectionDisplay) {
        if display == SectionDisplay::Scroll {
            let el: HtmlElement = self.iframe.content_document().unwrap_throw()
                .scrolling_element().unwrap_throw()
                .unchecked_into();

            el.style().remove_property("overflow").unwrap_throw();
        }
    }
}

impl PartialEq for ChapterContents {
    fn eq(&self, other: &Self) -> bool {
        self.chapter == other.chapter
    }
}


pub struct FoundChapterPage<'a> {
    pub chapter: &'a ChapterContents,
    pub local_page: usize
}

impl<'a> PartialEq for FoundChapterPage<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.chapter == other.chapter
    }
}