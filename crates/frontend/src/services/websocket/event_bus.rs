use common_local::ws::WebsocketNotification;
use std::collections::HashSet;
use yew_agent::worker::{HandlerId, Worker, WorkerScope};

pub struct WsEventBus {
    subscribers: HashSet<HandlerId>,
}

impl Worker for WsEventBus {
    type Message = ();
    type Input = WebsocketNotification;
    type Output = WebsocketNotification;

    fn create(_link: &WorkerScope<Self>) -> Self {
        Self {
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _scope: &WorkerScope<Self>, _msg: Self::Message) {
        //
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, _id: HandlerId) {
        for handler_id in self.subscribers.iter().copied() {
            scope.respond(handler_id, msg.clone());
        }
    }

    fn connected(&mut self, _scope: &WorkerScope<Self>, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, _scope: &WorkerScope<Self>, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}
