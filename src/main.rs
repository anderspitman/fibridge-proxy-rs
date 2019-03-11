use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use warp::filters::ws::{Message, WebSocket};
use futures::{Future, Stream, Sink};
use futures::sync::mpsc;
use omnistreams::{Multiplexer, MultiplexerEvent, EventEmitter, Transport};
use serde_json::json;
use uuid::Uuid;

type OmniMessage = Vec<u8>;
type MessageRx = mpsc::UnboundedReceiver<OmniMessage>;
type MessageTx = mpsc::UnboundedSender<OmniMessage>;
type Responses = Arc<Mutex<HashMap<String, i32>>>;

struct WebSocketTransport {
    out_tx: MessageTx,
    in_rx: Option<MessageRx>,
}

impl WebSocketTransport {
    fn new(ws: WebSocket) -> Self {
        let (ws_sink, ws_stream) = ws.split();
        let (out_tx, out_rx) = mpsc::unbounded::<OmniMessage>();
        let (in_tx, in_rx) = mpsc::unbounded::<OmniMessage>();

        let ws_sink = ws_sink
            .sink_map_err(|_e| ());

        let out_task = out_rx.map_err(|_e| ())
            .map(|omni_message| {
                Message::binary(omni_message)
            })
            .forward(ws_sink)
            .map(|_| ());

        warp::spawn(out_task);


        let in_tx = in_tx.sink_map_err(|_e| ());

        let in_task = ws_stream.map_err(|_e| ())
            .map(|ws_message| {
                ws_message.as_bytes().to_vec()
            })
            .forward(in_tx)
            .map(|_| ());

        warp::spawn(in_task);

        Self {
            out_tx,
            in_rx: Some(in_rx),
        }
    }
}

impl Transport for WebSocketTransport {
    fn send(&mut self, message: OmniMessage) {
        self.out_tx.unbounded_send(message).expect("ws transport send");
    }
    fn messages(&mut self) -> Option<mpsc::UnboundedReceiver<OmniMessage>> {
        Option::take(&mut self.in_rx)
    }
}

fn main() {
    let responses = Arc::new(Mutex::new(HashMap::new()));

    let omnis = warp::path("omnistreams")
        .and(warp::ws2())
        .map(|ws: warp::ws::Ws2| {
            ws.on_upgrade(|socket| {

                let transport = WebSocketTransport::new(socket);
                let mut mux = Multiplexer::new(transport);

                let id = Uuid::new_v4();

                let handshake_string = json!({
                    "type": "complete-handshake",
                    "id": id,
                }).to_string();

                println!("{:?}", handshake_string);

                mux.send_control_message(handshake_string.as_bytes().to_vec());

                let events = mux.events().expect("no events");

                warp::spawn(events.for_each(|event| {

                    match event {
                        MultiplexerEvent::ControlMessage(control_message) => {
                            println!("got control message: {:?}", control_message);
                        }
                        MultiplexerEvent::Conduit(_producer) => {
                        }
                    }
                    Ok(())
                }));

                futures::future::ok(())
            })
        });

    let download = warp::path::param()
        .and(warp::path::param())
        .map(move |id: String, filename: String| {
            println!("id: {}, filename: {}", id, filename);

            responses.lock().expect("get lock").insert(id, filename);

            "hi there"
        });

    let index = warp::path::end().map(|| {
        warp::reply::html("<h1>Hi there</h1>")
    });

    let routes = index
        .or(omnis)
        .or(download);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 9001));
}
