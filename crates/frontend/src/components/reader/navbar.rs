use std::{rc::Rc, sync::Mutex};

use common_local::{DisplayBookItem, MediaItem, Progression};
use yew::prelude::*;
use yew_router::prelude::Link;

use crate::BaseRoute;

#[derive(Properties)]
pub struct Property {
    // Callbacks
    // pub event: Callback<NavbarEvent>,
    pub file: Rc<MediaItem>,
    pub book: Rc<DisplayBookItem>,
    pub progress: Rc<Mutex<Option<Progression>>>,
}

impl PartialEq for Property {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.file, &other.file) && Rc::ptr_eq(&self.progress, &other.progress)
    }
}

#[function_component]
pub fn ReaderNavbar(props: &Property) -> Html {
    html! {
        <div class="d-flex flex-column text-bg-dark p-2">
            <div class="container-fluid">
                <div class="row">
                    <div class="col">
                        <Link<BaseRoute>
                            classes="btn btn-sm btn-secondary me-1"
                            to={ BaseRoute::ViewBook { book_id: props.book.id } }
                        ><i class="bi bi-arrow-left"></i></Link<BaseRoute>>

                        <button
                            type="button"
                            class="btn btn-sm btn-secondary me-1"
                            title="Chapters"
                            disabled=true
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
                                props.book.title.clone()
                                    .or_else(|| props.book.original_title.clone())
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
        </div>
    }
}
