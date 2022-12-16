use std::rc::Rc;

use common_local::ws::{TaskInfo, TaskType, WebsocketNotification};
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
            _producer: {
                let cb = {
                    let link = ctx.link().clone();
                    move |e| link.send_message(e)
                };

                WsEventBus::bridge(Rc::new(cb))
            },
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            WebsocketNotification::TaskStart { id, name } => {
                RUNNING_TASKS.lock().unwrap().insert(
                    id,
                    TaskInfo {
                        name,
                        current: None,
                    },
                );
            }

            WebsocketNotification::TaskUpdate {
                id,
                type_of,
                inserting,
            } => {
                if let Some(info) = RUNNING_TASKS.lock().unwrap().get_mut(&id) {
                    if inserting {
                        info.current = Some(type_of);
                    } else {
                        info.current = None;
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

                <div class="container-lg justify-content-md-center">
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
                                            <div>
                                                <h4>{ task.name.clone() }</h4>

                                                {
                                                    for task.current.clone()
                                                        .map(|type_of| html! {
                                                            <p>{ render_type_of(type_of) }</p>
                                                        })
                                                }

                                                <br />
                                            </div>
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

fn render_type_of(type_of: TaskType) -> String {
    match type_of {
        TaskType::UpdatingBook { id, subtitle } => {
            subtitle.unwrap_or_else(|| format!("Updating {id:?}"))
        }

        TaskType::LibraryScan(file_name) => file_name,
    }
}
