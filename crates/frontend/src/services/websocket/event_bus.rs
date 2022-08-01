use common_local::ws::WebsocketNotification;
use std::collections::HashSet;
use yew_agent::{Agent, AgentLink, Context, HandlerId};

pub struct WsEventBus {
    link: AgentLink<WsEventBus>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for WsEventBus {
    type Reach = Context<Self>;
    type Message = ();
    type Input = WebsocketNotification;
    type Output = WebsocketNotification;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        for handler_id in self.subscribers.iter().copied() {
            self.link.respond(handler_id, msg.clone());
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}