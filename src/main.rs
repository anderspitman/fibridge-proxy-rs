mod transport;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use warp::filters::ws::{WebSocket};
use futures::{Stream};
use omnistreams::{Multiplexer, MultiplexerEvent, EventEmitter};
use serde_json::json;
use uuid::Uuid;
use transport::WebSocketTransport;

//type Responses = Arc<Mutex<HashMap<String, i32>>>;

struct HosterManager {
}

impl HosterManager {
    fn new(ws: WebSocket) -> Self {

        let transport = WebSocketTransport::new(ws);
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

        Self {
        }
    }
}

fn main() {
    let responses = Arc::new(Mutex::new(HashMap::new()));

    let omnis = warp::path("omnistreams")
        .and(warp::ws2())
        .map(|ws: warp::ws::Ws2| {
            ws.on_upgrade(|socket| {
                
                let _hoster = HosterManager::new(socket);

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
