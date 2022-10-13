use common_local::ws::WebsocketResponse;
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use gloo_timers::future::TimeoutFuture;
use gloo_utils::window;
use reqwasm::websocket::{futures::WebSocket, Message};

use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use yew_agent::Dispatched;

mod event_bus;
pub use event_bus::WsEventBus;

use crate::util::as_local_path_without_http;

pub fn open_websocket_conn() {
    let ws_type = if window().location().protocol().unwrap_throw() == "https" {
        "wss"
    } else {
        "ws"
    };

    let url = format!("{ws_type}://{}", as_local_path_without_http("/ws/"));

    let ws = WebSocket::open(&url).unwrap();

    log::info!("Connected to WS: {}", url);

    // Split Websocket
    let (write, read) = ws.split();

    // Create Channel. Currently used for Ping/Pong.
    let (send, recieve) = channel::<WebsocketResponse>(1000);

    create_outgoing_writer(write, recieve);
    create_incoming_reader(read, send);
}

fn create_outgoing_writer(
    mut write: SplitSink<WebSocket, Message>,
    mut receive: Receiver<WebsocketResponse>,
) {
    spawn_local(async move {
        while let Some(s) = receive.next().await {
            if !s.is_pong() {
                log::debug!("WEBSOCKET [OUTGOING]: {:?}", s);
            }

            write
                .send(Message::Text(serde_json::to_string(&s).unwrap()))
                .await
                .unwrap();
        }

        log::debug!("WebSocket Send Closed");
    });
}

fn create_incoming_reader(
    mut read: SplitStream<WebSocket>,
    mut send_back: Sender<WebsocketResponse>,
) {
    let mut event_bus = WsEventBus::dispatcher();

    spawn_local(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(data)) => {
                    let resp: WebsocketResponse = serde_json::from_str(&data).unwrap();

                    if resp.is_ping() {
                        send_back.send(WebsocketResponse::Pong).await.unwrap();
                    } else if let WebsocketResponse::Notification(v) = resp {
                        log::debug!("WEBSOCKET [INCOMING] Text: {:?}", v);
                        event_bus.send(v);
                    }
                }

                Ok(Message::Bytes(b)) => {
                    let decoded = std::str::from_utf8(&b);

                    if let Ok(val) = decoded {
                        log::debug!("WEBSOCKET [INCOMING] Bytes: {}", val);
                    }
                }

                Err(e) => {
                    log::error!("Websocket: {:?}", e);
                    send_back.close_channel();

                    TimeoutFuture::new(10_000).await;

                    open_websocket_conn();

                    break;
                }
            }
        }

        log::debug!("WebSocket Receive Closed");
    });
}
