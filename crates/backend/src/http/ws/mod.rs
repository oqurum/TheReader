// Websocket used for notifications

use std::{
    sync::Mutex,
    time::{Duration, Instant}, collections::HashMap,
};

use actix::{Actor, ActorContext, AsyncContext, Handler, Message, Recipient, StreamHandler};
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use common_local::ws::{WebsocketNotification, WebsocketResponse, TaskId, TaskInfo};
use lazy_static::lazy_static;
use tracing::{info, trace};

lazy_static! {
    // TODO: Change lock type.
    static ref SOCKET_CLIENTS: Mutex<Vec<Recipient<Line>>> = Mutex::new(Vec::new());

    pub static ref RUNNING_TASKS: Mutex<HashMap<TaskId, TaskInfo>> = Mutex::new(HashMap::new());
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

struct MyWs {
    hb: Instant,
}

impl MyWs {
    fn new() -> Self {
        Self { hb: Instant::now() }
    }

    fn on_start(&self, ctx: &mut <Self as Actor>::Context) {
        // Save Client into array.
        SOCKET_CLIENTS
            .lock()
            .unwrap()
            .push(ctx.address().recipient());
    }

    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                trace!("Websocket Client heartbeat failed, disconnecting!");

                ctx.stop();

                return;
            }

            send_message(ctx, WebsocketResponse::Ping);
        });
    }
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.on_start(ctx);
        self.hb(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        // Remove client from array.

        let mut clients = SOCKET_CLIENTS.lock().unwrap();
        let weak = ctx.address().recipient();

        if let Some(index) = clients.iter().position(|x| x == &weak) {
            clients.remove(index);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                let resp: WebsocketResponse = serde_json::from_str(&text).unwrap();

                if resp.is_pong() {
                    self.hb = Instant::now();
                } else {
                    info!("WS Unknown: {:?}", resp);
                }
            }

            Ok(ws::Message::Binary(bin)) => {
                info!("WS Binary: {:?}", bin);
            }

            _ => (),
        }
    }
}

#[get("/ws/")]
pub async fn ws_index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(MyWs::new(), &req, stream)
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Line(WebsocketNotification);

impl Handler<Line> for MyWs {
    type Result = ();

    fn handle(&mut self, msg: Line, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_string(&WebsocketResponse::Notification(msg.0)).unwrap());
    }
}

fn send_message(ctx: &mut ws::WebsocketContext<MyWs>, value: WebsocketResponse) {
    ctx.text(serde_json::to_string(&value).unwrap());
}

pub fn send_message_to_clients(value: WebsocketNotification) {
    let clients = SOCKET_CLIENTS.lock().unwrap();

    for client in clients.as_slice() {
        client.do_send(Line(value.clone()));
    }

    match value {
        WebsocketNotification::TaskStart { id, name } => {
            RUNNING_TASKS.lock().unwrap().insert(id, TaskInfo {
                name,
                current: None,
            });
        }

        WebsocketNotification::TaskUpdate { id, type_of, inserting } => {
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
}
