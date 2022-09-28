use std::{collections::HashMap, rc::Rc, sync::Mutex, path::PathBuf};

use common_local::{MediaItem, Progression, Chapter, api, FileId};
use wasm_bindgen::{JsCast, prelude::{wasm_bindgen, Closure}};
use web_sys::HtmlIFrameElement;
use yew::{prelude::*, html::Scope};

use crate::request;



#[wasm_bindgen(module = "/js_generate_pages.js")]
extern "C" {
    // TODO: Sometimes will be 0. Example: if cover image is larger than body height. (Need to auto-resize.)
    fn get_iframe_page_count(iframe: &HtmlIFrameElement) -> usize;

    fn js_get_current_byte_pos(iframe: &HtmlIFrameElement) -> Option<usize>;
    fn js_get_page_from_byte_position(iframe: &HtmlIFrameElement, position: usize) -> Option<usize>;

    fn js_update_iframe_after_load(iframe: &HtmlIFrameElement, chapter: usize, handle_js_redirect_clicks: &Closure<dyn FnMut(usize, String)>);
    fn js_set_page_display_style(iframe: &HtmlIFrameElement, display: u8);
}



#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PageLoadType {
    All,
    Select,
}


#[derive(Clone, PartialEq, Eq)]
pub struct PageLoadSettings {
    pub speed: usize,
    pub type_of: PageLoadType,
}



#[derive(Clone, Copy, PartialEq, Eq)]
struct CachedPage {
    chapter: usize,
    chapter_local_page: usize,
}

struct CachedChapter {
    /// Page index that this chapter starts at.
    index: usize,
    /// Total Pages inside chapter
    pages: usize,
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


#[derive(Properties)]
pub struct Property {
    pub settings: PageLoadSettings,

    // Callbacks
    pub request_chapters: Callback<()>,

    pub book: Rc<MediaItem>,
    pub chapters: Rc<Mutex<LoadedChapters>>,

    pub progress: Rc<Mutex<Option<Progression>>>,
    pub display: ChapterDisplay,

    pub dimensions: (i32, i32),
    // pub ratio: (usize, usize)
}


impl PartialEq for Property {
    fn eq(&self, _other: &Self) -> bool {
        // TODO
        false
    }
}


pub enum TouchMsg {
    Start(i32, i32),
    End(i32, i32),
    Cancel
}


pub enum Msg {
    GenerateIFrameLoaded(GenerateChapter),

    // Event
    HandleJsRedirect(usize, String, Option<String>),

    Touch(TouchMsg),
    NextPage,
    PreviousPage,
    SetPage(usize),
    Ignore
}


pub struct Reader {
    // Cached from External Source
    cached_display: ChapterDisplay,
    cached_dimensions: Option<(i32, i32)>,

    loaded_sections: HashMap<usize, BookSection>,

    /// All pages which we've registered.
    cached_pages: Vec<CachedPage>,
    /// Position of the chapter based off the page index.
    cached_chapter_pos: HashMap<usize, CachedChapter>,

    /// The Total page we're currently on
    total_page_position: usize,

    /// The Chapter we're in
    viewing_chapter: usize,
    // TODO: Decide if we want to keep. Not really needed since we can aquire it based off of self.cached_pages[self.total_page_position].chapter

    handle_js_redirect_clicks: Closure<dyn FnMut(usize, String)>,

    handle_touch_start: Option<Closure<dyn FnMut(TouchEvent)>>,
    handle_touch_end: Option<Closure<dyn FnMut(TouchEvent)>>,
    handle_touch_cancel: Option<Closure<dyn FnMut(TouchEvent)>>,

    touch_start: Option<(i32, i32)>
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
            cached_display: ctx.props().display,
            cached_dimensions: None,
            loaded_sections: {
                let mut map = HashMap::new();

                for i in 0..ctx.props().book.chapter_count {
                    map.insert(i, BookSection::Waiting);
                }

                map
            },

            cached_pages: Vec::new(),
            cached_chapter_pos: HashMap::new(),

            total_page_position: 0,
            viewing_chapter: 0,

            handle_js_redirect_clicks,

            handle_touch_cancel: None,
            handle_touch_end: None,
            handle_touch_start: None,

            touch_start: None,
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
                    self.set_chapter(chap.value);
                    // TODO: Handle id_name
                }
            }

            Msg::SetPage(new_page) => {
                return self.set_page(new_page.min(self.page_count().saturating_sub(1)), ctx);
            }

            Msg::NextPage => {
                if self.total_page_position + 1 == self.page_count() {
                    return false;
                }

                self.set_page(self.total_page_position + 1, ctx);
            }

            Msg::PreviousPage => {
                if self.total_page_position == 0 {
                    return false;
                }

                self.set_page(self.total_page_position - 1, ctx);
            }

            Msg::Touch(msg) => match msg {
                TouchMsg::Start(point_x, point_y) => {
                    self.touch_start = Some((point_x, point_y));
                    return false;
                }

                TouchMsg::End(end_x, end_y) => {
                    if let Some((start_x, start_y)) = self.touch_start {
                        let (dist_x, dist_y) = (start_x - end_x, start_y - end_y);

                        // Are we dragging vertically or horizontally?
                        if dist_x.abs() > dist_y.abs() {
                            if dist_x.abs() > 100 {
                                log::info!("Changing Page");

                                if dist_x.is_positive() {
                                    log::info!("Next Page");
                                    ctx.link().send_message(Msg::NextPage);
                                } else {
                                    log::info!("Previous Page");
                                    ctx.link().send_message(Msg::PreviousPage);
                                }
                            }
                        } else {
                            // TODO: Vertical
                        }

                        self.touch_start = None;
                    }

                    return false;
                }

                TouchMsg::Cancel => {
                    self.touch_start = None;
                    return false;
                }
            }

            // Called after iframe is loaded.
            Msg::GenerateIFrameLoaded(page) => {
                js_update_iframe_after_load(&page.iframe, page.chapter.value, &self.handle_js_redirect_clicks);

                {
                    let gen = self.loaded_sections.remove(&page.chapter.value).unwrap();
                    self.loaded_sections.insert(page.chapter.value, gen.convert_to_loaded());
                }

                let loading_count = self.loaded_sections.values().filter(|v| v.is_loading()).count();

                if self.are_all_sections_generated() {
                    self.on_all_frames_generated(ctx);
                }

                self.update_cached_pages();

                self.use_progression(*ctx.props().progress.lock().unwrap());

                if loading_count == 0 {
                    ctx.props().request_chapters.emit(());
                }
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let page_count = self.page_count();

        let pages_style = format!("width: {}px; height: {}px;", ctx.props().dimensions.0, ctx.props().dimensions.1);

        let progress_precentage = format!("width: {}%;", (self.total_page_position + 1) as f64 / page_count as f64 * 100.0);

        html! {
            <div class="reader">
                <div class="navbar">
                    <a onclick={ctx.link().callback(|_| Msg::SetPage(0))}>{"First Page"}</a>
                    <a onclick={ctx.link().callback(|_| Msg::PreviousPage)}>{"Previous Page"}</a>

                    {
                        match ctx.props().settings.type_of {
                            PageLoadType::All => html! {
                                <>
                                    <span>{ "Page " } { self.total_page_position + 1 } { "/" } { page_count }</span>
                                    <a onclick={ ctx.link().callback(|_| Msg::NextPage) }>{ "Next Page" }</a>
                                    <a onclick={ ctx.link().callback(move |_| Msg::SetPage(page_count - 1)) }>{ "Last Page" }</a>
                                </>
                            },

                            PageLoadType::Select => html! {
                                <>
                                    <span>{ "Section " } { self.viewing_chapter + 1 } { "/" } { ctx.props().book.chapter_count }</span>
                                    <a onclick={ ctx.link().callback(|_| Msg::NextPage) }>{ "Next Page" }</a>
                                </>
                            }
                        }
                    }
                </div>

                <div class="pages" style={pages_style.clone()}>
                    <div class="frames" style={format!("top: -{}%;", self.viewing_chapter * 100)}>
                        {
                            for (0..ctx.props().book.chapter_count)
                                .into_iter()
                                .map(|i| {
                                    if let Some(v) = self.loaded_sections.get(&i).unwrap().as_chapter() {
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
                <div class="progress"><div class="prog-bar" style={progress_precentage}></div></div>
            </div>
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let props = ctx.props();

        if self.cached_display != props.display || self.cached_dimensions != Some(props.dimensions) {
            self.cached_display = props.display;
            self.cached_dimensions = Some(props.dimensions);

            for chap in self.loaded_sections.values() {
                if let BookSection::Loaded(chap) = chap {
                    js_set_page_display_style(&chap.iframe, props.display.into());
                    update_iframe_size(Some(props.dimensions), &chap.iframe);
                }
            }

            self.update_cached_pages();
        }

        // TODO: Move to Msg::GenerateIFrameLoaded so it's only in a single place.
        self.use_progression(*props.progress.lock().unwrap());

        // Continue loading chapters
        let chaps = props.chapters.lock().unwrap();

        // Reverse iterator since for some reason chapter "generation" works from LIFO
        for chap in chaps.chapters.iter().rev() {
            if let Some(sec) = self.loaded_sections.get_mut(&chap.value) {
                if sec.is_waiting() {
                    log::info!("Generating Chapter {}", chap.value + 1);

                    *sec = BookSection::Loading(generate_pages(
                        Some(props.dimensions),
                        props.book.id,
                        chap.clone(),
                        ctx.link().clone()
                    ));
                }
            }
        }

        true
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let window = gloo_utils::window();

            // TODO: Touch Start is being called 24 times at once.
            // TODO: Not working for my Samsung Tablet with Firefox.

            let link = ctx.link().clone();
            let handle_touch_start = Closure::wrap(Box::new(move |event: TouchEvent| {
                let touches = event.touches();

                if touches.length() == 1 {
                    let touch = touches.get(0).unwrap();
                    link.send_message(Msg::Touch(TouchMsg::Start(touch.client_x(), touch.client_y())));
                }
            }) as Box<dyn FnMut(TouchEvent)>);

            let link = ctx.link().clone();
            let handle_touch_end = Closure::wrap(Box::new(move |event: TouchEvent| {
                let touches = event.touches();

                if touches.length() == 1 {
                    let touch = touches.get(0).unwrap();
                    link.send_message(Msg::Touch(TouchMsg::End(touch.client_x(), touch.client_y())));
                }
            }) as Box<dyn FnMut(TouchEvent)>);

            let link = ctx.link().clone();
            let handle_touch_cancel = Closure::wrap(Box::new(move |_: TouchEvent| {
                link.send_message(Msg::Touch(TouchMsg::Cancel));
            }) as Box<dyn FnMut(TouchEvent)>);

            window.add_event_listener_with_callback("touchstart", handle_touch_start.as_ref().unchecked_ref()).unwrap();
            window.add_event_listener_with_callback("touchend", handle_touch_end.as_ref().unchecked_ref()).unwrap();
            window.add_event_listener_with_callback("touchcancel", handle_touch_cancel.as_ref().unchecked_ref()).unwrap();

            self.handle_touch_cancel = Some(handle_touch_cancel);
            self.handle_touch_end = Some(handle_touch_end);
            self.handle_touch_start = Some(handle_touch_start);
        }
    }
}

impl Reader {
    fn use_progression(&mut self, prog: Option<Progression>) {
        if let Some(prog) = prog {
            match prog {
                Progression::Ebook { chapter, char_pos, .. } if self.viewing_chapter == 0 => {
                    if self.loaded_sections.contains_key(&(chapter as usize)) {
                        // TODO: utilize page. Main issue is resizing the reader w/h will return a different page. Hence the char_pos.
                        self.set_chapter(chapter as usize);

                        if char_pos != -1 {
                            let chapter = self.loaded_sections.get(&self.viewing_chapter).unwrap();

                            if let BookSection::Loaded(chapter) = chapter {
                                let page = js_get_page_from_byte_position(&chapter.iframe, char_pos as usize);

                                if let Some(page) = page {
                                    chapter.set_page(page);
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
        self.loaded_sections.values().all(|v| v.is_loaded())
    }

    fn update_cached_pages(&mut self) {
        let current_section_page = self.cached_pages.get(self.total_page_position).cloned();

        self.cached_pages.clear();

        let mut total_page_pos = 0;

        // TODO: Verify if needed. Or can we do values_mut() we need to have it in asc order
        for chap in 0..self.loaded_sections.len() {
            if let Some(BookSection::Loaded(ele)) = self.loaded_sections.get_mut(&chap) {
                let page_count = get_iframe_page_count(&ele.iframe).max(1);

                if let Some(cached_chap) = self.cached_chapter_pos.get_mut(&chap) {
                    cached_chap.index = total_page_pos;
                    cached_chap.pages = page_count;
                } else {
                    self.cached_chapter_pos.insert(chap, CachedChapter {
                        index: total_page_pos,
                        pages: page_count,
                    });
                }

                total_page_pos += page_count;

                for local_page in 0..page_count {
                    self.cached_pages.push(CachedPage {
                        chapter: ele.chapter,
                        chapter_local_page: local_page
                    });
                }
            }
        }

        // We have to reset our global page position
        if let Some(current_page) = current_section_page {
            if let Some(idx) = self.cached_pages.iter().skip(self.total_page_position.saturating_sub(1)).position(|v| *v == current_page) {
                self.total_page_position = idx;
            }
        }
    }

    fn on_all_frames_generated(&mut self, ctx: &Context<Self>) {
        log::info!("All Frames Generated");
        // Double check page counts before proceeding.
        self.update_cached_pages();

        // TODO: Move to Msg::GenerateIFrameLoaded so it's only in a single place.
        self.use_progression(*ctx.props().progress.lock().unwrap());
    }

    fn set_page(&mut self, new_total_page: usize, ctx: &Context<Self>) -> bool {
        if let Some(page) = self.cached_pages.get(new_total_page) {
            self.total_page_position = new_total_page;
            self.viewing_chapter = page.chapter;

            log::debug!("set_page chapter: {} {}", self.viewing_chapter, page.chapter_local_page);

            if let BookSection::Loaded(chap) = self.loaded_sections.get(&self.viewing_chapter).unwrap() {
                chap.set_page(page.chapter_local_page);
                self.upload_progress(&chap.iframe, ctx);

                ctx.props().request_chapters.emit(());
            }

            true
        } else {
            false
        }
    }

    fn set_chapter(&mut self, new_chapter: usize) -> bool {
        if let Some(chap) = self.cached_chapter_pos.get(&new_chapter) {
            self.total_page_position = chap.index;
            self.viewing_chapter = new_chapter;

            true
        } else {
            false
        }
    }

    pub fn page_count(&self) -> usize {
        self.cached_pages.len()
    }

    fn upload_progress(&self, iframe: &HtmlIFrameElement, ctx: &Context<Self>) {
        let (chapter, page, char_pos, book_id) = (
            self.viewing_chapter as i64,
            self.total_page_position as i64,
            js_get_current_byte_pos(iframe).map(|v| v as i64).unwrap_or(-1),
            ctx.props().book.id
        );

        let last_page = self.page_count().saturating_sub(1);

        let stored_prog = Rc::clone(&ctx.props().progress);

        match page {
            0 if chapter == 0 => *stored_prog.lock().unwrap() = None,

            // TODO: Figure out what the last page of each book actually is.
            v if v as usize == last_page => *stored_prog.lock().unwrap() = Some(Progression::Complete),

            _ => {
                let progression = Progression::Ebook {
                    char_pos,
                    chapter,
                    page
                };

                *stored_prog.lock().unwrap() = Some(progression);
            }
        }

        ctx.link()
        .send_future(async move {
            match page {
                0 if chapter == 0 => {
                    request::remove_book_progress(book_id).await;
                }

                // TODO: Figure out what the last page of each book actually is.
                v if v as usize == last_page => {
                    request::update_book_progress(book_id, &Progression::Complete).await;
                }

                _ => {
                    let progression = Progression::Ebook {
                        char_pos,
                        chapter,
                        page
                    };

                    request::update_book_progress(book_id, &progression).await;
                }
            }

            Msg::Ignore
        });
    }
}




#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChapterDisplay {
    Single = 0,
    Double = 1,
    Scroll = 2,
}

impl From<u8> for ChapterDisplay {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Single,
            1 => Self::Double,
            2 => Self::Scroll,
            _ => unimplemented!()
        }
    }
}


impl From<ChapterDisplay> for u8 {
    fn from(val: ChapterDisplay) -> Self {
        val as u8
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
        chapter: chap_value,
        iframe: new_frame,
        on_load: f,
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

    pub fn as_chapter(&self) -> Option<&ChapterContents> {
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
    pub chapter: usize,
    pub iframe: HtmlIFrameElement,
}

impl ChapterContents {
    pub fn set_page(&self, page_number: usize) {
        let body = self.iframe.content_document().unwrap().body().unwrap();
        body.style().set_property("left", &format!("calc(-{}% - {}px)", 100 * page_number, page_number * 10)).unwrap();
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