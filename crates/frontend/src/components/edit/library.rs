use common::component::PopupClose;
use common_local::{api::UpdateLibrary, LibraryId};
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;
use yew_hooks::{use_async, use_async_with_options, UseAsyncOptions};

use crate::request;

#[derive(PartialEq, Properties)]
pub struct LibraryEditProperty {
    pub id: LibraryId,
    /// When we update the Library.
    pub on_change: Option<Callback<UpdateLibrary>>,
}

#[function_component(LibraryEdit)]
pub fn _lib_edit(prop: &LibraryEditProperty) -> Html {
    let update = yew_hooks::use_update();

    let lib_id = prop.id;

    let library_update = yew::use_mut_ref(UpdateLibrary::default);

    let resp = use_async_with_options(
        async move { request::get_library(lib_id).await.ok() },
        UseAsyncOptions::enable_auto(),
    );

    let on_change_name = {
        let library_update = library_update.clone();
        let update = update.clone();

        Callback::from(move |e: Event| {
            let mut borrow = library_update.borrow_mut();
            borrow.name = Some(
                e.target_unchecked_into::<HtmlInputElement>()
                    .value()
                    .trim()
                    .to_string(),
            )
            .filter(|v| !v.is_empty());

            // Drop is needed since update() refreshes the state instantly and we're still borrowing.
            drop(borrow);

            update();
        })
    };

    let on_add_directory = {
        let library_update = library_update.clone();
        let resp = resp.clone();
        let update = update.clone();

        Callback::from(move |value: String| {
            let mut borrow = library_update.borrow_mut();

            if borrow.add_directories.iter().any(|v| v == &value) {
                return;
            }

            if let Some(index) = borrow.remove_directories.iter().position(|v| v == &value) {
                borrow.remove_directories.swap_remove(index);
            } else if let Some(resp) = resp.data.as_ref() {
                if resp.directories.iter().any(|v| v == &value) {
                    return;
                }
            } else {
                borrow.add_directories.push(value);
            }

            drop(borrow);

            update();
        })
    };

    let on_remove_directory = {
        let library_update = library_update.clone();

        Callback::from(move |value: String| {
            let mut borrow = library_update.borrow_mut();

            if let Some(index) = borrow.add_directories.iter().position(|v| v == &value) {
                borrow.add_directories.remove(index);
            } else {
                borrow.remove_directories.push(value);
            }

            drop(borrow);

            update();
        })
    };

    let on_submit = if let Some(cb) = prop.on_change.as_ref() {
        let library_update = library_update.clone();

        cb.reform(move |_| library_update.borrow().clone())
    } else {
        let library_update = library_update.clone();

        let func = use_async(async move {
            request::update_library(lib_id, &library_update.take())
                .await
                .ok()
        });

        Callback::from(move |_| func.run())
    };

    html! {
        <>
            <div class="modal-header">
                <h3 class="modal-title">{ "Editing Library" }</h3>
            </div>

            <div class="modal-body">
                // Loading
                {
                    if resp.loading {
                        html! {
                            <h4>{ "Loading..." }</h4>
                        }
                    } else {
                        html! {}
                    }
                }

                // Error
                {
                    // TODO: Check if I can .take() instead.
                    if let Some(err) = resp.error.as_ref() {
                        html! {
                            <>
                                <h4>{ "Error:" }</h4>
                                <span>{ err.description.clone() }</span>
                            </>
                        }
                    } else {
                        html! {}
                    }
                }

                // Response
                {
                    if let Some(library) = resp.data.as_ref() {
                        html! {
                            <>
                                <div class="mb-3">
                                    <label class="form-label" for="asdf">{ "Library Name: " }</label>
                                    <input class="form-control" placeholder="Library Name" type="text" value={ library.name.clone() } onchange={ on_change_name } />
                                </div>

                                <div class="mb-3">
                                    <h5>{ "Directories:" }</h5>
                                    <ul>
                                        {
                                            for library.directories.iter()
                                                .cloned()
                                                .map(|path| {
                                                    let path2 = path.clone();
                                                    let on_remove_directory = on_remove_directory.clone();
                                                    let on_add_directory = on_add_directory.clone();

                                                    html! {
                                                        <li>
                                                            {
                                                                if library_update.borrow().remove_directories.iter().any(|i| i == &path) {
                                                                    html! {
                                                                        <button
                                                                            class="btn btn-success btn-sm"
                                                                            onclick={ Callback::from(move |_| on_add_directory.emit(path2.clone())) }
                                                                        >
                                                                            { "+" }
                                                                        </button>
                                                                    }
                                                                } else {
                                                                    html! {
                                                                        <button
                                                                            class="btn btn-danger btn-sm"
                                                                            onclick={ Callback::from(move |_| on_remove_directory.emit(path2.clone())) }
                                                                        >
                                                                            { "X" }
                                                                        </button>
                                                                    }
                                                                }
                                                            }

                                                            { path }
                                                        </li>
                                                    }
                                                })
                                        }
                                    </ul>
                                </div>

                                <NewLibraryDirectory id={ library.id } callback={ on_add_directory } />
                            </>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
            <div class="modal-footer">
                // TODO: Don't close the popup until we received a response.
                <PopupClose><button
                    class="btn btn-success btn-sm"
                    onclick={ on_submit }
                >{ "Submit" }</button></PopupClose>
                <PopupClose><button class="btn btn-danger btn-sm">{ "Cancel" }</button></PopupClose>
            </div>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct NewLibraryDirectoryProps {
    pub callback: Callback<String>,

    pub id: LibraryId,
}

#[function_component(NewLibraryDirectory)]
fn new_library_dir(props: &NewLibraryDirectoryProps) -> Html {
    let directory = use_state(String::new);

    let on_create = {
        let directory = directory.clone();

        props.callback.reform(move |_| {
            let value = directory.to_string();

            directory.set(String::new());

            value
        })
    };

    let on_change_lib_name = {
        let directory = directory.setter();

        Callback::from(move |e: Event| {
            directory.set(e.target_unchecked_into::<HtmlInputElement>().value());
        })
    };

    html! {
        <>
            <h5>{ "Add Directory to Library" }</h5>

            <div class="input-group mb-3">
                // TODO: Use the FileSearch Component
                <input class="form-control" type="text" name="directory-name" placeholder="Directory" onchange={ on_change_lib_name } />

                <button class="btn btn-success btn-sm" onclick={ on_create }>{ "Create" }</button>
            </div>
        </>
    }
}
