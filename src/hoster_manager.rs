use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use futures::sync::{mpsc, oneshot};
use futures::{Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use omnistreams::{
    Multiplexer, MultiplexerEvent, EventEmitter, Producer, ProducerEvent,
    Streamer, CancelReason,
};
use super::transport::WebSocketTransport;
use warp::http::{Response};
use hyper::Body;
use warp::filters::ws::{WebSocket};


//type ResponseTx = mpsc::UnboundedSender<Vec<u8>>;
type ResponseTx = oneshot::Sender<Response<Body>>;
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
                    let message: Value = serde_json::from_slice(&control_message)
                        .expect("parse control message");

                    println!("control message");
                    println!("{:?}", message);

                    if message["type"] == "error" {
                        println!("error");

                        match &message["requestId"] {
                            Value::Number(request_id) => {
                                let request_id = request_id.as_u64().expect("parse u64") as usize;
                                let mut lock = response_txs_clone.lock().expect("get lock");
                                let response_tx = lock.remove(&request_id).expect("removed tx");

                                let response = Response::builder()
                                    .status(404)
                                    .body("Not found".into()).expect("error response");

                                response_tx.send(response).unwrap();
                            },
                            _ => (),
                        }
                    }
                }
                MultiplexerEvent::Conduit(mut producer, metadata) => {

                    let md: ConduitMetadata = serde_json::from_slice(&metadata).expect("parse metadata");

                    println!("Create conduit");
                    println!("{:?}", md);

                    let request_id = md.id;

                    let (stream_tx, stream_rx) = mpsc::unbounded::<Vec<u8>>();
                    let stream_rx = stream_rx.map_err(|_e| {
                        "stream fail"
                    });

                    // See if there's a way to do this without importing hyper
                    // directly.
                    let body = Body::wrap_stream(stream_rx);

                    let response = Response::builder()
                        .body(body).expect("response");


                    let mut lock = response_txs_clone.lock().expect("get lock");
                    let response_tx = lock.remove(&request_id).expect("removed tx");
                    response_tx.send(response).unwrap();

                    let events = producer.event_stream().expect("producer events");

                    producer.request(1);

                    warp::spawn(events.for_each(move |event| {

                        match event {
                            ProducerEvent::Data(data) => {
                                producer.request(1);
                                match stream_tx.unbounded_send(data) {
                                    Ok(_) => {
                                    },
                                    Err(_) => {
                                        eprintln!("dc");
                                        producer.cancel(CancelReason::Disconnected);
                                    }
                                };
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

    pub fn process_request(&mut self, filename: String, range_header: String) -> oneshot::Receiver<Response<Body>> {

        let request_id = self.next_request_id();

        println!("filename: {}", filename);


        let mut request = json!({
            "type": "GET",
            "url": format!("/{}", filename),
            "requestId": request_id,
        });

        let range = parse_range_header(&range_header);

        if range.is_some() {
            request["range"] = range.unwrap();
        }

        self.mux.send_control_message(request.to_string().as_bytes().to_vec());
        

        let (response_tx, response_rx) = oneshot::channel();

        self.response_txs.lock().expect("get lock").insert(request_id, response_tx);

        response_rx
    }
}

fn parse_range_header(header: &str) -> Option<Value> {
    if header == "" {
        return None;
    }
    else {

        // TODO: error handling
        let parts: Vec<&str> = header.split('=').collect();
        let range_str: Vec<&str> = parts[1].split('-').collect();
        let start = range_str[0].parse::<usize>().expect("parse range start");
        let end_str = range_str[1];

        let mut range = json!({
            "start": start,
        });

        if end_str.len() > 0 {
            let end = end_str.parse::<usize>().expect("parse range end");
            range["end"] = json!(end);
        }

        Some(range)
    }
}
