mod transport;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use warp::filters::ws::{WebSocket};
use warp::http::Response;
use futures::{Stream};
use futures::sync::mpsc;
use omnistreams::{Multiplexer, MultiplexerEvent, EventEmitter};
use serde_json::json;
use uuid::Uuid;
use transport::WebSocketTransport;
use hyper::Body;

type HosterManagers = Arc<Mutex<HashMap<String, mpsc::UnboundedReceiver<Vec<u8>>>>>;

struct HosterManager {
    id: String,
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
            id: id.to_string(),
        }
    }
}

fn main() {
    let hoster_managers = Arc::new(Mutex::new(HashMap::new()));
    let hoster_managers_clone = hoster_managers.clone();

    let omnis = warp::path("omnistreams")
        .map(move || hoster_managers.clone())
        .and(warp::ws2())
        .map(|hoster_managers: HosterManagers, ws: warp::ws::Ws2| {
            ws.on_upgrade(move |socket| {

                let (tx, rx) = mpsc::unbounded::<Vec<u8>>();
                
                let hoster = HosterManager::new(socket);

                tx.unbounded_send("Hi there".as_bytes().to_vec());
                tx.unbounded_send("Ya fuzzy little manpeach".as_bytes().to_vec());

                hoster_managers.lock().expect("get lock").insert(hoster.id, rx);

                futures::future::ok(())
            })
        });

    let download = warp::path::param()
        .and(warp::path::param())
        .map(move |id: String, filename: String| {
            println!("id: {}, filename: {}", id, filename);

            let stream = hoster_managers_clone.lock()
                .expect("get lock").remove(&id)
                .expect("get hoster manager");

            let stream = stream.map_err(|_e| {
                "stream fail"
            });

            // See if there's a way to do this without importing hyper
            // directly.
            let body = Body::wrap_stream(stream);

            Response::builder()
                .body(body)
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
