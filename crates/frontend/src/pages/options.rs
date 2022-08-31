use common_local::{api::{self, RunTaskBody}, BasicLibrary, LibraryId};
use common::{component::popup::{Popup, PopupType}, api::WrappingResponse};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::use_list;

use crate::{request, get_member_self};

pub enum Msg {
    // Request Results
    OptionsResults(WrappingResponse<api::GetOptionsResponse>),

    // Events
    DisplayPopup(usize, LibraryId),
    ClosePopup,

    RequestUpdateOptions(bool, api::ModifyOptionsBody),

    Ignore
}

pub struct OptionsPage {
    resp: Option<api::GetOptionsResponse>,
    visible_popup: Option<(usize, LibraryId)>,
}

impl Component for OptionsPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            resp: None,
            visible_popup: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::OptionsResults(resp) => {
                match resp.ok() {
                    Ok(resp) => {
                        self.resp = Some(resp);
                        self.visible_popup = None;
                    },
                    Err(err) => crate::display_error(err),
                }
            }

            Msg::DisplayPopup(popup, index) => {
                self.visible_popup = Some((popup, index));
            }

            Msg::ClosePopup => {
                self.visible_popup = None;
            }

            Msg::RequestUpdateOptions(is_adding, options) => {
                ctx.link().send_future(async move {
                    if is_adding {
                        request::update_options_add(options).await;
                    } else {
                        request::update_options_remove(options).await;
                    }

                    Msg::OptionsResults(request::get_options().await)
                });
            }

            Msg::Ignore => ()
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let member = get_member_self().unwrap();

        if let Some(resp) = self.resp.as_ref() {
            html! {
                <div class="options-page">
                    <h2>{ "Tasks" }</h2>

                    <br />

                    <button onclick={ ctx.link().callback_future(|_| async {
                        request::run_task(RunTaskBody {
                            run_search: true,
                            run_metadata: false,
                        }).await;
                        Msg::Ignore
                    }) }>{ "Run Library Scan" }</button>

                    <button onclick={ ctx.link().callback_future(|_| async {
                        request::run_task(RunTaskBody {
                            run_search: false,
                            run_metadata: true,
                        }).await;
                        Msg::Ignore
                    }) }>{ "Update Metadata" }</button>

                    <br />

                    <h2>{ "Libraries" }</h2>
                    {
                        for resp.libraries.iter()
                            .map(|v| {
                                let lib_id = v.id;

                                html! {
                                    <>
                                        <h3>{ v.name.clone() }</h3>
                                        <button class="red" onclick={ ctx.link().callback(move|_| {
                                            Msg::RequestUpdateOptions(
                                                false,
                                                api::ModifyOptionsBody {
                                                    library: Some(BasicLibrary {
                                                        id: Some(lib_id),
                                                        name: None,
                                                        directories: None,
                                                    }),
                                                }
                                            )
                                        }) }>{ "delete" }</button>
                                        <ul>
                                            {
                                                for v.directories.iter().map(move |v| {
                                                    let path = v.clone();

                                                    html! {
                                                        <li><button class="red" onclick={ ctx.link().callback(move |_| {
                                                            Msg::RequestUpdateOptions(
                                                                false,
                                                                api::ModifyOptionsBody {
                                                                    library: Some(BasicLibrary {
                                                                        id: Some(lib_id),
                                                                        name: None,
                                                                        directories: Some(vec![path.clone()]),
                                                                    }),
                                                                }
                                                            )
                                                        }) }>{ "X" }</button>{ v.clone() }</li>
                                                    }
                                                })
                                            }
                                            <li><button class="green" onclick={ctx.link().callback(move |_| Msg::DisplayPopup(1, lib_id))}>{ "Add New" }</button></li>
                                        </ul>
                                    </>
                                }
                            })
                    }
                    <button class="green" onclick={ctx.link().callback(|_| Msg::DisplayPopup(0, LibraryId::none()))}>{ "Add Library" }</button>

                    {
                        if member.permissions.is_owner() {
                            let config = resp.config.as_ref().unwrap();

                            html! {
                                <>
                                    <br />

                                    <h2>{ "Admin" }</h2>

                                    <h4>{ "External Metadata Server" }</h4>
                                    {
                                        if config.libby.token.is_some() {
                                            html! {
                                                <span class="label green">
                                                    { "Metadata Server Link is Setup: " }
                                                    <a href={ config.libby.url.clone() }>{ config.libby.url.clone() }</a>
                                                </span>
                                            }
                                        } else {
                                            html! {
                                                <>
                                                    <span class="label red">
                                                        { "Click below to setup the Metadata Server Link for " }
                                                        <b><a href={ config.libby.url.clone() }>{ config.libby.url.clone() }</a></b>
                                                    </span>
                                                    <br />
                                                    <form action="/api/setup/agent" method="POST">
                                                        <button class="green" type="submit">{ "Link" }</button>
                                                    </form>
                                                </>
                                            }
                                        }
                                    }
                                </>
                            }
                        } else {
                            html! {}
                        }
                    }

                    { self.render_popup(ctx) }
                </div>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link()
            .send_future(async {
                Msg::OptionsResults(request::get_options().await)
            });
        }
    }
}

impl OptionsPage {
    fn render_popup(&self, ctx: &Context<Self>) -> Html {
        if let Some((popup_id, item_index)) = self.visible_popup {
            // TODO: Make popup component for this.

            match popup_id {
                // Add Library
                0 => html! {
                    <NewLibrary callback={ ctx.link().callback(|v| v) } />
                },

                // Add Directory to Library
                1 => html! {
                    <NewLibraryDirectory callback={ ctx.link().callback(|v| v) } library_id={ item_index } />
                },

                _ => html! {}
            }

        } else {
            html! {}
        }
    }
}



#[derive(Properties, PartialEq)]
pub struct NewLibraryProps {
    pub callback: Callback<Msg>,
}

#[function_component(NewLibrary)]
fn new_library(props: &NewLibraryProps) -> Html {
    let library_name = use_state(String::new);
    let directories = use_list(Vec::<String>::new());

    let on_create = {
        let dirs = directories.clone();
        let name = library_name.clone();

        props.callback.reform(move |_| {
            Msg::RequestUpdateOptions(
                true,
                api::ModifyOptionsBody {
                    library: Some(BasicLibrary {
                        id: None,
                        name: Some(name.to_string()),
                        directories: Some(dirs.current().to_vec()),
                    }),
                }
            )
        })
    };

    let on_new_dir_path = {
        let dirs = directories.clone();
        Callback::from(move |e: KeyboardEvent| if e.key() == "Enter" {
            let input = e.target_unchecked_into::<HtmlInputElement>();

            dirs.push(input.value());

            input.set_value("");
        })
    };

    let on_change_lib_name = {
        let name = library_name.setter();

        Callback::from(move |e: Event| {
            name.set(e.target_unchecked_into::<HtmlInputElement>().value());
        })
    };


    html! {
        <Popup
            // classes=""
            type_of={ PopupType::FullOverlay }
            on_close={ props.callback.reform(|_| Msg::ClosePopup) }
        >
            <h2>{ "New Library" }</h2>

            <div class="form-container">
                <div class="row">
                    <input type="text" name="library-name" placeholder="Library Name" onchange={ on_change_lib_name } />
                    <button class="green" onclick={ on_create }>{"Create"}</button>
                </div>
            </div>

            <h5>{ "Directories" }</h5>

            <div class="form-container">
                {
                    for directories.current()
                        .iter()
                        .enumerate()
                        .map(|(index, path)| {
                            let dirs = directories.clone();

                            let onclick = Callback::from(move |_| { dirs.remove(index); });

                            html! {
                                <div class="row">
                                    <button class="red" {onclick}>{ "X" }</button>
                                    <span>{ path.clone() }</span>
                                </div>
                            }
                        }
                    )
                }

                <input
                    onkeypress={ on_new_dir_path }
                />
            </div>
        </Popup>
    }
}



#[derive(Properties, PartialEq)]
pub struct NewLibraryDirectoryProps {
    pub callback: Callback<Msg>,

    pub library_id: LibraryId,
}

#[function_component(NewLibraryDirectory)]
fn new_library_dir(props: &NewLibraryDirectoryProps) -> Html {
    let directory = use_state(String::new);

    let on_create = {
        let directory = directory.clone();
        let library_id = props.library_id;

        props.callback.reform(move |_| {
            Msg::RequestUpdateOptions(
                true,
                api::ModifyOptionsBody {
                    library: Some(BasicLibrary {
                        id: Some(library_id),
                        name: None,
                        directories: Some(vec![directory.to_string()]),
                    }),
                }
            )
        })
    };

    let on_change_lib_name = {
        let directory = directory.setter();

        Callback::from(move |e: Event| {
            directory.set(e.target_unchecked_into::<HtmlInputElement>().value());
        })
    };


    html! {
        <Popup
            // classes=""
            type_of={ PopupType::FullOverlay }
            on_close={ props.callback.reform(|_| Msg::ClosePopup) }
        >
            <h2>{ "Add Directory to Library" }</h2>

            <div class="form-container">
                <div class="row">
                    // TODO: Directory Selector
                    <input type="text" name="directory-name" placeholder="Directory" onchange={ on_change_lib_name } />

                    <button class="green" onclick={ on_create }>{"Create"}</button>
                </div>
            </div>
        </Popup>
    }
}