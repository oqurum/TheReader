use common::{component::CarouselComponent, api::WrappingResponse};
use common_local::api::{ApiGetBookPresetListResponse, BookPresetListType};
use yew::prelude::*;

use crate::{components::{BookPosterItem, Sidebar}, request};

pub enum Msg {
    Ignore,

    ProgressResponse(ApiGetBookPresetListResponse),
}

pub struct HomePage {
    section_progressing: Option<ApiGetBookPresetListResponse>
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

        Self {
            section_progressing: None,
        }
    }


    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => return false,
            Msg::ProgressResponse(v) => self.section_progressing = Some(v),
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <Sidebar />
                <div class="view-container">
                    {
                        if let Some(sec) = self.section_progressing.as_ref() {
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
                </div>
            </div>
        }
    }
}