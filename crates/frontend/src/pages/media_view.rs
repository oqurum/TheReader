use common_local::{api::{MediaViewResponse, self}, util::file_size_bytes_to_readable_string};
use common::{BookId, component::popup::{Popup, PopupClose, PopupType}};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{request, Route, components::{PopupSearchBook, PopupEditMetadata}, util::{on_click_prevdef, on_click_prevdef_stopprop}};

#[derive(Clone)]
pub enum Msg {
    // Retrive
    RetrieveMediaView(Box<MediaViewResponse>),

    // Events
    ShowPopup(DisplayOverlay),
    ClosePopup,

    // Popup Events
    UpdateMeta(BookId),

    Ignore
}

#[derive(Properties, PartialEq)]
pub struct Property {
    pub id: BookId
}

pub struct MediaView {
    media: Option<MediaViewResponse>,

    media_popup: Option<DisplayOverlay>,
}

impl Component for MediaView {
    type Message = Msg;
    type Properties = Property;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            media: None,
            media_popup: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ignore => return false,

            Msg::ClosePopup => {
                self.media_popup = None;
            }

            Msg::RetrieveMediaView(value) => {
                self.media = Some(*value);
            }

            Msg::ShowPopup(new_disp) => {
                if let Some(old_disp) = self.media_popup.as_mut() {
                    if *old_disp == new_disp {
                        self.media_popup = None;
                    } else {
                        self.media_popup = Some(new_disp);
                    }
                } else {
                    self.media_popup = Some(new_disp);
                }
            }

            Msg::UpdateMeta(meta_id) => {
                ctx.link()
                .send_future(async move {
                    request::update_metadata(meta_id, &api::PostMetadataBody::AutoMatchMetaIdBySource).await;

                    Msg::Ignore
                });
            }
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="outer-view-container">
                <div class="sidebar-container"></div>
                <div class="view-container">
                    { self.render_main(ctx) }
                </div>
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let metadata_id = ctx.props().id;

            ctx.link().send_future(async move {
                Msg::RetrieveMediaView(Box::new(request::get_media_view(metadata_id).await))
            });
        }
    }
}

impl MediaView {
    fn render_main(&self, ctx: &Context<Self>) -> Html {
        if let Some(MediaViewResponse { people, metadata, media, progress }) = self.media.as_ref() {
            let meta_id = metadata.id;
            let on_click_more = ctx.link().callback(move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();

                Msg::ShowPopup(DisplayOverlay::More { meta_id, mouse_pos: (e.page_x(), e.page_y()) })
            });

            let media_prog = media.iter().zip(progress.iter());

            html! {
                <div class="item-view-container">
                    <div class="info-container">
                        <div class="poster large">
                            <div class="bottom-right">
                                <span class="material-icons" onclick={on_click_more} title="More Options">{ "more_horiz" }</span>
                            </div>
                            <div class="bottom-left">
                                <span class="material-icons" onclick={ctx.link().callback_future(move |e: MouseEvent| {
                                    e.prevent_default();
                                    e.stop_propagation();

                                    async move {
                                        Msg::ShowPopup(DisplayOverlay::Edit(Box::new(request::get_media_view(meta_id).await)))
                                    }
                                })} title="More Options">{ "edit" }</span>
                            </div>

                            <img src={ metadata.get_thumb_url() } />
                        </div>
                        <div class="metadata-container">
                            <div class="metadata">
                                <h3 class="title">{ metadata.get_title() }</h3>
                                <p class="description">{ metadata.description.clone().unwrap_or_default() }</p>
                            </div>
                        </div>
                    </div>

                    <section>
                        <h2>{ "Files" }</h2>
                        <div class="files-container">
                            {
                                for media_prog.map(|(media, _prog)| {
                                    html! {
                                        <Link<Route> to={Route::ReadBook { book_id: media.id }} classes={ classes!("file-item") }>
                                            <h5>{ media.file_name.clone() }</h5>
                                            <div><b>{ "File Size: " }</b>{ file_size_bytes_to_readable_string(media.file_size) }</div>
                                            <div><b>{ "File Type: " }</b>{ media.file_type.clone() }</div>
                                        </Link<Route>>
                                    }
                                })
                            }
                        </div>
                    </section>

                    <section>
                        <h2>{ "Characters" }</h2>
                        <div class="characters-container">
                            <div class="person-item">
                                <div class="photo"><img src="/images/missingperson.jpg" /></div>
                                <span class="title">{ "Character #1" }</span>
                            </div>
                            <div class="person-item">
                                <div class="photo"><img src="/images/missingperson.jpg" /></div>
                                <span class="title">{ "Character #2" }</span>
                            </div>
                        </div>
                    </section>

                    <section>
                        <h2>{ "People" }</h2>
                        <div class="authors-container">
                            {
                                for people.iter().map(|person| {
                                    html! {
                                        <div class="person-item">
                                            <div class="photo"><img src={ person.get_thumb_url() } /></div>
                                            <span class="title">{ person.name.clone() }</span>
                                        </div>
                                    }
                                })
                            }
                        </div>
                    </section>

                    {
                        if let Some(overlay_type) = self.media_popup.as_ref() {
                            match overlay_type {
                                DisplayOverlay::Info { meta_id: _ } => {
                                    html! {
                                        <Popup type_of={ PopupType::FullOverlay } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <h1>{"Info"}</h1>
                                        </Popup>
                                    }
                                }

                                &DisplayOverlay::More { meta_id, mouse_pos } => {
                                    html! {
                                        <Popup type_of={ PopupType::AtPoint(mouse_pos.0, mouse_pos.1) } on_close={ctx.link().callback(|_| Msg::ClosePopup)}>
                                            <div class="menu-list">
                                                <PopupClose class="menu-item">{ "Start Reading" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef(ctx.link(), Msg::UpdateMeta(meta_id))
                                                }>{ "Refresh Metadata" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop(ctx.link(), Msg::ShowPopup(DisplayOverlay::SearchForBook { meta_id, input_value: None }))
                                                }>{ "Search For Book" }</PopupClose>
                                                <PopupClose class="menu-item">{ "Delete" }</PopupClose>
                                                <PopupClose class="menu-item" onclick={
                                                    on_click_prevdef_stopprop(ctx.link(), Msg::ShowPopup(DisplayOverlay::Info { meta_id }))
                                                }>{ "Show Info" }</PopupClose>
                                            </div>
                                        </Popup>
                                    }
                                }

                                DisplayOverlay::Edit(resp) => {
                                    html! {
                                        <PopupEditMetadata
                                            on_close={ ctx.link().callback(|_| Msg::ClosePopup) }
                                            classes={ classes!("popup-book-edit") }
                                            media_resp={ (&**resp).clone() }
                                        />
                                    }
                                }

                                &DisplayOverlay::SearchForBook { meta_id, ref input_value } => {
                                    let input_value = if let Some(v) = input_value {
                                        v.to_string()
                                    } else {
                                        format!(
                                            "{} {}",
                                            metadata.title.as_deref().unwrap_or_default(),
                                            metadata.cached.author.as_deref().unwrap_or_default()
                                        )
                                    };

                                    let input_value = input_value.trim().to_string();

                                    html! {
                                        <PopupSearchBook {meta_id} {input_value} on_close={ ctx.link().callback(|_| Msg::ClosePopup) } />
                                    }
                                }
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
            }
        } else {
            html! {
                <h1>{ "Loading..." }</h1>
            }
        }
    }
}


#[derive(Clone)]
pub enum DisplayOverlay {
    Info {
        meta_id: BookId
    },

    Edit(Box<api::MediaViewResponse>),

    More {
        meta_id: BookId,
        mouse_pos: (i32, i32)
    },

    SearchForBook {
        meta_id: BookId,
        input_value: Option<String>,
    },
}

impl PartialEq for DisplayOverlay {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Info { meta_id: l_id }, Self::Info { meta_id: r_id }) => l_id == r_id,
            (Self::More { meta_id: l_id, .. }, Self::More { meta_id: r_id, .. }) => l_id == r_id,
            (
                Self::SearchForBook { meta_id: l_id, input_value: l_val, .. },
                Self::SearchForBook { meta_id: r_id, input_value: r_val, .. }
            ) => l_id == r_id && l_val == r_val,

            _ => false
        }
    }
}