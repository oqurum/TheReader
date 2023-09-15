use common_local::Progression;
use yew::{prelude::*, use_state_eq};
use yew_router::prelude::Link;

use crate::{components::reader::UpdatableReadingInfo, BaseRoute};

use super::ReadingInfo;

#[derive(PartialEq, Eq)]
enum Viewing {
    None,
    TableOfContents,
    Bookmarks,
}

#[function_component]
pub fn ReaderNavbar() -> Html {
    let reading_info = use_context::<UpdatableReadingInfo>().unwrap();

    let displaying = use_state_eq(|| Viewing::None);

    let borrow = reading_info.borrow();

    html! {
        <div class="d-flex flex-column text-bg-dark p-2">
            <div class="container-fluid">
                <div class="row">
                    <div class="col">
                        <Link<BaseRoute>
                            classes="btn btn-sm btn-secondary me-1"
                            to={ BaseRoute::ViewBook { book_id: borrow.get_book().id } }
                        ><i class="bi bi-arrow-left"></i></Link<BaseRoute>>

                        <button
                            type="button"
                            class={ classes!("btn", "btn-sm", "btn-secondary", "me-1", (*displaying == Viewing::TableOfContents).then_some("active")) }
                            title="Chapters"
                            onclick={
                                let setter = displaying.clone();

                                Callback::from(move |_| {
                                    if *setter == Viewing::TableOfContents {
                                        setter.set(Viewing::None);
                                    } else {
                                        setter.set(Viewing::TableOfContents);
                                    }
                                })
                            }
                        ><i class="bi bi-list-stars"></i></button>

                        <button
                            type="button"
                            class="btn btn-sm btn-secondary"
                            title="Bookmarks"
                            disabled=true
                        ><i class="bi bi-bookmark-heart"></i></button>

                        // [settable page] of [page count]
                        // <div>
                        //     <input
                        //         class="form-control form-control-sm"
                        //         type="text"
                        //         value="1"
                        //     />
                        //     { " of " }
                        //     // Page Count
                        // </div>
                    </div>
                    <div class="col text-center">
                        <span class="align-middle">
                            {
                                borrow.get_book().title.clone()
                                    .or_else(|| borrow.get_book().original_title.clone())
                                    .unwrap_or_default()
                            }
                        </span>
                    </div>
                    <div class="col text-end">
                        // zoom level
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary me-1"
                            title="Fullscreen"
                            disabled=true
                        ><i class="bi bi-arrows-fullscreen"></i></button>
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary me-1"
                            title="Find"
                            disabled=true
                        ><i class="bi bi-search-heart"></i></button>
                        <div class="btn-group">
                            <button
                                class="btn btn-secondary btn-sm dropdown-toggle"
                                type="button"
                                data-bs-toggle="dropdown"
                                title="More"
                                disabled=true
                            >
                                <i class="bi bi-three-dots"></i>
                            </button>
                            <ul class="dropdown-menu">
                                <li><a class="dropdown-item" href="#">{ "Option #1" }</a></li>
                                <li><hr class="dropdown-divider" /></li>
                                <li><a class="dropdown-item" href="#">{ "Settings" }</a></li>
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
            {
                match *displaying {
                    Viewing::None => html! {},
                    Viewing::Bookmarks => html! {},
                    Viewing::TableOfContents => render_toc(&reading_info),
                }
            }
        </div>
    }
}

fn render_toc(updatable_info: &UpdatableReadingInfo) -> Html {
    let reading_info = updatable_info.borrow();

    html! {
        <div
            class="text-bg-dark d-flex flex-column px-1 py-2 overflow-auto position-absolute"
            style="left: 0; top: 47px; width: var(--sidebar-width); height: calc(100% - 47px);"
        >
            {
                if reading_info.table_of_contents.is_empty() {
                    html! {
                        <h3>{ "No TOCs" }</h3>
                    }
                } else {
                    html! {
                        for reading_info.table_of_contents.iter().map(|&(ref name, section)| {
                            html! {
                                <button
                                    type="button"
                                    class="btn btn-secondary btn-sm mb-1"
                                    onclick={
                                        updatable_info.reform(move |_, state| {
                                            if let Some(Progression::Ebook {
                                                chapter,
                                                char_pos,
                                                page,
                                            }) = state.progress.as_mut()
                                            {
                                                *chapter = section as i64;
                                                *char_pos = -1;
                                                *page = -1;
                                            }
                                        })
                                    }
                                >{ name }</button>
                            }
                        })
                    }
                }
            }
        </div>
    }
}
