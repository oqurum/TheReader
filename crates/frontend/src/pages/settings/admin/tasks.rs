use common_local::ws::{WebsocketNotification, TaskType, TaskInfo};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{services::WsEventBus, RUNNING_TASKS};

pub struct AdminTaskPage {
    _producer: Box<dyn Bridge<WsEventBus>>,
}

impl Component for AdminTaskPage {
    type Message = WebsocketNotification;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            _producer: WsEventBus::bridge(ctx.link().callback(|v| v)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        log::debug!("{msg:?}");

        match msg {
            WebsocketNotification::TaskStart { id, name } => {
                RUNNING_TASKS.lock().unwrap().insert(id, TaskInfo { name, updating: Vec::new(), subtitle: Vec::new() });
            }

            WebsocketNotification::TaskUpdate { id, type_of, inserting, subtitle } => {
                if let TaskType::UpdatingBook(book_id) = type_of {
                    if let Some(task_items) = RUNNING_TASKS.lock().unwrap().get_mut(&id) {
                        if inserting {
                            task_items.subtitle.push(subtitle);
                            task_items.updating.push(book_id);
                        } else if let Some(index) = task_items.updating.iter().position(|v| v == &book_id) {
                            task_items.subtitle.swap_remove(index);
                            task_items.updating.swap_remove(index);
                        }
                    }
                }
            }

            WebsocketNotification::TaskEnd(id) => {
                RUNNING_TASKS.lock().unwrap().remove(&id);
            }
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        // let member = get_member_self().unwrap();

        let tasks = RUNNING_TASKS.lock().unwrap();

        html! {
            <div class="view-container">
                <h2>{ "Tasks" }</h2>

                <br />

                <div class="row justify-content-md-center">
                    <div class="p-3 col-md-auto bg-dark">
                        {
                            if tasks.is_empty() {
                                html! {
                                    <h4>{ "Nothing Running" }</h4>
                                }
                            } else {
                                html! {
                                    for tasks.values()
                                        .map(|task| html! {
                                            <>
                                                <h4>{ task.name.clone() }</h4>

                                                {
                                                    for task.updating.iter().zip(task.subtitle.iter())
                                                        .map(|(book_id, subtitle)| html! {
                                                            <div class="alert alert-secondary">
                                                                { subtitle.clone().unwrap_or_else(|| format!("Updating {book_id:?}")) }
                                                            </div>
                                                        })
                                                }

                                                <br />
                                            </>
                                        })
                                }
                            }
                        }
                    </div>
                </div>
            </div>
        }
    }
}
