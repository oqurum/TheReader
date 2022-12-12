use common_local::{
    LibraryId,
};
use yew::prelude::*;

use crate::{
    components::{
        BookListComponent,
        BookListRequest,
    },
    request,
    util::build_book_filter_query,
};

#[derive(Properties, PartialEq, Eq)]
pub struct Property {
    pub id: LibraryId,
}

pub struct LibraryPage;

impl Component for LibraryPage {
    type Message = ();
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let library = ctx.props().id;

        html! {
            <div class="view-container">
                <BookListComponent on_load={ ctx.link().callback_future(move |v: BookListRequest| async move {
                    let res = request::get_books(
                        Some(library),
                        v.offset,
                        None,
                        Some(build_book_filter_query()),
                    )
                    .await;

                    v.response.emit(res);
                }) } />
            </div>
        }
    }
}