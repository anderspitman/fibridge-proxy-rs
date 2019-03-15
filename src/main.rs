mod transport;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use warp::filters::ws::{WebSocket};
use warp::http::Response;
use futures::{Stream};
use futures::sync::mpsc;
use omnistreams::{Multiplexer, MultiplexerEvent, EventEmitter, Producer, ProducerEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use self::transport::WebSocketTransport;
use hyper::Body;

type HosterManagers = Arc<Mutex<HashMap<String, HosterManager>>>;

#[derive(Serialize, Deserialize, Debug)]
struct ConduitMetadata {
    id: usize,
    size: usize,
}

struct HosterManager {
    id: String,
    response_tx: mpsc::UnboundedSender<Vec<u8>>,
    next_request_id: usize,
    mux: Multiplexer,
}

impl HosterManager {
    fn new(ws: WebSocket, response_tx: mpsc::UnboundedSender<Vec<u8>>) -> Self {

        response_tx.unbounded_send("Hi there".as_bytes().to_vec());
        response_tx.unbounded_send("Ya fuzzy little manpeach".as_bytes().to_vec());

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

        warp::spawn(events.for_each(move |event| {

            match event {
                MultiplexerEvent::ControlMessage(control_message) => {
                    println!("got control message: {:?}", std::str::from_utf8(&control_message).expect("parse utf"));
                }
                MultiplexerEvent::Conduit(mut producer, metadata) => {
                    println!("got producer");

                    let md: ConduitMetadata = serde_json::from_slice(&metadata).expect("parse metadata");

                    println!("{:?}", md);

                    let events = producer.event_stream().expect("producer events");

                    producer.request(1);

                    warp::spawn(events.for_each(move |event| {

                        match event {
                            ProducerEvent::Data(data) => {
                                producer.request(1);
                            },
                            ProducerEvent::End => {
                            },
                        }
                        Ok(())
                    }));
                }
            }
            Ok(())
        }));

        Self {
            id: id.to_string(),
            response_tx,
            next_request_id: 0,
            mux,
        }
    }

    fn next_request_id(&mut self) -> usize {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    fn process_request(&mut self, filename: String) -> String {

        let request_id = self.next_request_id();

        println!("filename: {}", filename);

        let request = json!({
            "type": "GET",
            "url": format!("/{}", filename),
            "requestId": request_id,
        }).to_string();

        self.mux.send_control_message(request.as_bytes().to_vec());
        "Hi there".into()
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
                
                let hoster = HosterManager::new(socket, tx);

                hoster_managers.lock().expect("get lock").insert(hoster.id.clone(), hoster);

                futures::future::ok(())
            })
        });

    let download = warp::path::param()
        .and(warp::path::param())
        .map(move |id: String, filename: String| {
            println!("id: {}, filename: {}", id, filename);

            //let stream = hoster_managers_clone.lock()
            //    .expect("get lock").remove(&id)
            //    .expect("get hoster manager");

            //let stream = stream.map_err(|_e| {
            //    "stream fail"
            //});

            //// See if there's a way to do this without importing hyper
            //// directly.
            //let body = Body::wrap_stream(stream);

            //Response::builder()
            //    .body(body)

            let mut lock = hoster_managers_clone.lock().expect("get lock");
            let manager = lock.get_mut(&id).expect("get hoster manager");

            manager.process_request(filename)
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
