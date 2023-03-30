use std::{collections::HashMap, rc::Rc};

use common::{api::WrappingResponse, component::CarouselComponent};
use common_local::{
    api::{self, ApiGetBookPresetListResponse, BookPresetListType},
    filter::{FilterContainer, FilterTableType},
    DisplayItem, LibraryId,
};
use yew::prelude::*;

use crate::{components::BookPosterItem, request, AppState};

pub enum Msg {
    Ignore,

    ProgressResponse(ApiGetBookPresetListResponse),

    LibraryRecentResponse(LibraryId, WrappingResponse<api::GetBookListResponse>),

    ContextChanged(Rc<AppState>),
}

pub struct HomePage {
    state: Rc<AppState>,
    _listener: ContextHandle<Rc<AppState>>,

    section_progressing: Option<ApiGetBookPresetListResponse>,

    library_items: HashMap<LibraryId, Vec<DisplayItem>>,

    is_requesting: bool,
}

impl Component for HomePage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(async {
            match request::get_books_preset(None, None, BookPresetListType::Progressing).await {
                WrappingResponse::Resp(resp) => Msg::ProgressResponse(resp),
                WrappingResponse::Error(err) => {
                    crate::display_error(err);

                    Msg::Ignore
                }
            }
        });

        let (state, _listener) = ctx
            .link()
            .context::<Rc<AppState>>(ctx.link().callback(Msg::ContextChanged))
            .expect("context to be set");


        let mut this = Self {
            state,
            _listener,

            section_progressing: None,

            library_items: HashMap::default(),
            is_requesting: false,
        };

        this.load_next_library(ctx);

        this
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ContextChanged(state) => {
                self.state = state;

                self.load_next_library(ctx);
            }

            Msg::Ignore => return false,

            Msg::ProgressResponse(v) => self.section_progressing = Some(v),

            Msg::LibraryRecentResponse(library_id, resp) => {
                self.is_requesting = false;

                match resp.ok() {
                    Ok(resp) => {
                        self.library_items.insert(library_id, resp.items);
                    }
                    Err(e) => crate::display_error(e),
                }

                self.load_next_library(ctx);
            }
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="view-container">
                {
                    if let Some(sec) = self.section_progressing.as_ref().filter(|v| !v.items.is_empty()) {
                        html! {
                            <>
                                <h3>{ "In Progress" }</h3>
                                <CarouselComponent>
                                {
                                    for sec.items.iter().map(|item| {
                                        html! {
                                            <BookPosterItem
                                                is_updating=false

                                                progress={ (item.progress, item.file.clone()) }
                                                item={ item.book.clone() }
                                            />
                                        }
                                    })
                                }
                                </CarouselComponent>
                            </>
                        }
                    } else {
                        html! {}
                    }
                }

                {
                    for self.state.libraries.iter()
                        .map(|lib| html! {
                            <>
                                <h3>{ format!("Recently Added In {}", lib.name) }</h3>
                                {
                                    if let Some(contents) = self.library_items.get(&lib.id) {
                                        html! {
                                            <CarouselComponent>
                                            {
                                                for contents.iter().cloned().map(|item| {
                                                    html! {
                                                        <BookPosterItem
                                                            is_updating=false

                                                            { item }
                                                        />
                                                    }
                                                })
                                            }
                                            </CarouselComponent>
                                        }
                                    } else {
                                        html! {
                                            <div>
                                                <span>{ "Loading..." }</span>
                                            </div>
                                        }
                                    }
                                }
                            </>
                        })
                }
            </div>
        }
    }
}

impl HomePage {
    pub fn load_next_library(&mut self, ctx: &Context<Self>) {
        if self.is_requesting {
            return;
        }

        for lib in &self.state.libraries {
            if !self.library_items.contains_key(&lib.id) {
                let lib_id = lib.id;

                self.is_requesting = true;

                ctx.link().send_future(async move {
                    Msg::LibraryRecentResponse(
                        lib_id,
                        request::get_books(
                            Some(lib_id),
                            None,
                            Some(25),
                            Some(
                                FilterContainer::default()
                                    .order_by(FilterTableType::CreatedAt, true),
                            ),
                        )
                        .await,
                    )
                });

                break;
            }
        }
    }
}
