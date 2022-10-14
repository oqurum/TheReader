use std::collections::HashMap;

use common::{api::WrappingResponse, component::CarouselComponent};
use common_local::{
    api::{self, ApiGetBookPresetListResponse, BookPresetListType},
    filter::{FilterContainer, FilterTableType},
    DisplayItem, LibraryColl, LibraryId,
};
use yew::prelude::*;

use crate::{
    components::{BookPosterItem, Sidebar},
    request,
};

pub enum Msg {
    Ignore,

    ProgressResponse(ApiGetBookPresetListResponse),

    LibraryRecentResponse(LibraryId, WrappingResponse<api::GetBookListResponse>),

    LibraryListResults(WrappingResponse<api::GetLibrariesResponse>),
}

pub struct HomePage {
    libraries: Vec<LibraryColl>,

    section_progressing: Option<ApiGetBookPresetListResponse>,

    library_items: HashMap<LibraryId, Vec<DisplayItem>>,
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

        ctx.link()
            .send_future(async move { Msg::LibraryListResults(request::get_libraries().await) });

        Self {
            libraries: Vec::new(),

            section_progressing: None,

            library_items: HashMap::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => return false,

            Msg::ProgressResponse(v) => self.section_progressing = Some(v),

            Msg::LibraryRecentResponse(library_id, resp) => {
                match resp.ok() {
                    Ok(resp) => {
                        self.library_items.insert(library_id, resp.items);
                    }
                    Err(e) => crate::display_error(e),
                }

                self.load_next_library(ctx);
            }

            Msg::LibraryListResults(resp) => {
                match resp.ok() {
                    Ok(resp) => self.libraries = resp.items,
                    Err(err) => crate::display_error(err),
                }

                self.load_next_library(ctx);
            }
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <Sidebar />
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
                                                    is_editing=false
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
                        for self.libraries.iter()
                            .map(|lib| html! {
                                <>
                                    <h3>{ lib.name.clone() }</h3>
                                    {
                                        if let Some(contents) = self.library_items.get(&lib.id) {
                                            html! {
                                                <CarouselComponent>
                                                {
                                                    for contents.iter().cloned().map(|item| {
                                                        html! {
                                                            <BookPosterItem
                                                                is_editing=false
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
            </div>
        }
    }
}

impl HomePage {
    pub fn load_next_library(&self, ctx: &Context<Self>) {
        for lib in &self.libraries {
            if !self.library_items.contains_key(&lib.id) {
                let lib_id = lib.id;

                ctx.link().send_future(async move {
                    Msg::LibraryRecentResponse(
                        lib_id,
                        request::get_books(
                            Some(lib_id),
                            None,
                            Some(25),
                            Some(FilterContainer::default().order_by(FilterTableType::CreatedAt, true)),
                        )
                        .await,
                    )
                });

                break;
            }
        }
    }
}
