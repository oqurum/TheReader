use common::component::PopupClose;
use common_local::{LibraryId, api::UpdateLibrary};
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;
use yew_hooks::{UseAsyncOptions, use_async_with_options, use_async};

use crate::request;




#[derive(PartialEq, Eq, Properties)]
pub struct LibraryEditProperty {
    pub id: LibraryId,
    // TODO: Optionally return UpdateLibrary (eg. for initial setup process)
}

#[function_component(LibraryEdit)]
pub fn _lib_edit(prop: &LibraryEditProperty) -> Html {
    let lib_id = prop.id;

    let library_update = yew::use_mut_ref(UpdateLibrary::default);

    let resp = use_async_with_options(
        async move {
            request::get_library(lib_id).await.ok()
        },
        UseAsyncOptions::enable_auto()
    );

    let on_change_name = {
        let library_update = library_update.clone();

        Callback::from(move |e: Event| {
            let mut borrow = library_update.borrow_mut();
            borrow.name = Some(e.target_unchecked_into::<HtmlInputElement>().value().trim().to_string()).filter(|v| !v.is_empty());
        })
    };

    let update_lib = use_async(async move {
        request::update_library(lib_id, &library_update.take()).await.ok()
    });

    html! {
        <div class="library-edit">
            <h3>{ "Editing Library" }</h3>

            {
                if resp.loading {
                    html! {
                        <h4>{ "Loading..." }</h4>
                    }
                } else {
                    html! {}
                }
            }

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

            {
                if let Some(library) = resp.data.as_ref() {
                    html! {
                        <>
                            <div class="form-container">
                                <label for="asdf">{ "Library Name: " }</label>
                                <input placeholder="Library Name" type="text" value={ library.name.clone() } onchange={ on_change_name } />
                            </div>

                            <div class="form-container row">
                                // TODO: Determine if we're in a popup. We may not be in one.
                                // TODO: Don't close the popup until we received a response.
                                <PopupClose><button
                                    class="green"
                                    onclick={ Callback::from(move |_| update_lib.run()) }
                                >{ "Submit" }</button></PopupClose>
                                <PopupClose><button class="red">{ "Cancel" }</button></PopupClose>
                            </div>
                        </>
                    }
                } else {
                    html! {}
                }
            }
        </div>
    }
}