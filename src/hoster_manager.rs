use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use futures::sync::mpsc;
use futures::{Stream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use omnistreams::{Multiplexer, MultiplexerEvent, EventEmitter, Producer, ProducerEvent};
use super::transport::WebSocketTransport;
use warp::http::Response;
use hyper::Body;
use warp::filters::ws::{WebSocket};


type ResponseTx = mpsc::UnboundedSender<Vec<u8>>;
type ResponseTxs = Arc<Mutex<HashMap<usize, ResponseTx>>>;

#[derive(Serialize, Deserialize, Debug)]
struct ConduitMetadata {
    id: usize,
    size: usize,
}

pub struct HosterManager {
    id: String,
    next_request_id: usize,
    mux: Multiplexer,
    response_txs: ResponseTxs,
}

impl HosterManager {
    pub fn new(ws: WebSocket) -> Self {

        let transport = WebSocketTransport::new(ws);
        let mut mux = Multiplexer::new(transport);

        let id = Uuid::new_v4();

        let handshake_string = json!({
            "type": "complete-handshake",
            "id": id,
        }).to_string();

        mux.send_control_message(handshake_string.as_bytes().to_vec());

        let events = mux.events().expect("no events");

        let response_txs: ResponseTxs = Arc::new(Mutex::new(HashMap::new()));
        let response_txs_clone = response_txs.clone();

        warp::spawn(events.for_each(move |event| {

            match event {
                MultiplexerEvent::ControlMessage(control_message) => {
                    println!("got control message: {:?}", std::str::from_utf8(&control_message).expect("parse utf"));
                }
                MultiplexerEvent::Conduit(mut producer, metadata) => {

                    let md: ConduitMetadata = serde_json::from_slice(&metadata).expect("parse metadata");

                    println!("Create conduit");
                    println!("{:?}", md);

                    let request_id = md.id;

                    let mut lock = response_txs_clone.lock().expect("get lock");
                    let response_tx = lock.remove(&request_id).expect("removed tx");

                    let events = producer.event_stream().expect("producer events");

                    producer.request(1);

                    warp::spawn(events.for_each(move |event| {

                        match event {
                            ProducerEvent::Data(data) => {
                                producer.request(1);
                                response_tx.unbounded_send(data).expect("response tx send");
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
            next_request_id: 0,
            mux,
            response_txs,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    fn next_request_id(&mut self) -> usize {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    pub fn process_request(&mut self, filename: String) -> Response<Body> {

        let request_id = self.next_request_id();

        println!("filename: {}", filename);

        let request = json!({
            "type": "GET",
            "url": format!("/{}", filename),
            "requestId": request_id,
        }).to_string();

        self.mux.send_control_message(request.as_bytes().to_vec());
        
        let (tx, rx) = mpsc::unbounded::<Vec<u8>>();

        self.response_txs.lock().expect("get lock").insert(request_id, tx);

        let rx = rx.map_err(|_e| {
            "stream fail"
        });

        // See if there's a way to do this without importing hyper
        // directly.
        let body = Body::wrap_stream(rx);

        Response::builder()
            .body(body).expect("response")
    }
}
