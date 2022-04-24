// Websocket used for notifications

use std::{sync::Mutex, time::{Instant, Duration}};

use actix::{Actor, StreamHandler, Recipient, Message, Handler, AsyncContext, ActorContext};
use actix_web::{web, Error, HttpRequest, HttpResponse, get};
use actix_web_actors::ws;
use lazy_static::lazy_static;


lazy_static! {
	static ref SOCKET_CLIENTS: Mutex<Vec<Recipient<Line>>> = Mutex::new(Vec::new());
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);


struct MyWs {
	hb: Instant,
}

impl MyWs {
	fn new() -> Self {
		Self {
			hb: Instant::now(),
		}
	}

	fn on_start(&self, ctx: &mut <Self as Actor>::Context) {
		// Save Client into array.
		SOCKET_CLIENTS.lock().unwrap().push(ctx.address().recipient());
	}

	fn hb(&self, ctx: &mut <Self as Actor>::Context) {
		ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
			if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
				log::debug!("Websocket Client heartbeat failed, disconnecting!");

				ctx.stop();

				return;
			}

			ctx.ping(&[]);
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

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[get("/ws/")]
pub async fn ws_index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(MyWs::new(), &req, stream);
    println!("{:?}", resp);
    resp
}



#[derive(Message)]
#[rtype(result = "()")]
pub struct Line(String);

impl Handler<Line> for MyWs {
    type Result = ();

    fn handle(&mut self, msg: Line, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}