use common::{component::PopupClose, MISSING_THUMB_PATH};
use yew::prelude::*;
use yew_router::prelude::Link;

use crate::BaseRoute;

// TODO: Better Names for both of these types.

#[derive(Clone, Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub image: Option<String>,
    #[prop_or_default]
    pub title: Option<String>,
    #[prop_or_default]
    pub subtitle: Option<String>,
    #[prop_or_default]
    pub description: Option<String>,

    #[prop_or_default]
    pub class: Option<String>,
    #[prop_or_default]
    pub small: Option<bool>,

    #[prop_or_default]
    pub onclick: Option<Callback<()>>,

    #[prop_or_default]
    pub onclick_close_popup: bool,
    #[prop_or_default]
    pub to: Option<BaseRoute>,
}

pub struct BookListItemInfo;

impl Component for BookListItemInfo {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let Props {
            image,
            title,
            subtitle,
            description,
            onclick,

            class,
            small,

            onclick_close_popup,
            to,
        } = ctx.props().clone();

        let contents = html! {
            <>
                <div class={ classes!("poster", small.unwrap_or_default().then_some("small")) }>
                    <img src={ image.unwrap_or_else(|| MISSING_THUMB_PATH.to_string()) } />
                </div>
                <div class="info">
                    <h5 class="title" title={ title.clone() }>{ title.unwrap_or_else(|| String::from("Untitled")) }</h5>

                    {
                        if let Some(subtitle) = subtitle {
                            html! {
                                <h6 class="subtitle" title={ subtitle.clone() }>{ subtitle }</h6>
                            }
                        } else {
                            html! {}
                        }
                    }

                    {
                        if let Some(description) = description {
                            html! {
                                <p class="description">{ description }</p>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
            </>
        };

        if let Some(to) = to {
            html! {
                <Link<BaseRoute> {to} classes={ classes!("book-list-item-info", class, small.unwrap_or_default().then_some("small")) }>
                    { contents }
                </Link<BaseRoute>>
            }
        } else if onclick_close_popup {
            html! {
                <PopupClose class={ classes!("book-list-item-info", class, small.unwrap_or_default().then_some("small")) } onclick={ onclick.map(|v| v.reform(|_| ())) }>
                    { contents }
                </PopupClose>
            }
        } else {
            html! {
                <div class={ classes!("book-list-item-info", class, small.unwrap_or_default().then_some("small")) } onclick={ onclick.map(|v| v.reform(|_| ())) }>
                    { contents }
                </div>
            }
        }
    }
}
