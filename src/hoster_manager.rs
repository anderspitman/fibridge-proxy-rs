use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use futures::sync::{mpsc, oneshot};
use futures::{Stream};
use serde_json::{json, Value};
use omnistreams::{
    Multiplexer, MultiplexerEvent, EventEmitter, Producer, SinkAdapter,
    MapConduit, Message,
};
use super::transport::WebSocketTransport;
use warp::http::{Response};
use hyper::Body;
use warp::filters::ws::{WebSocket};
use crate::stats_conduit::StatsConduit;


type ResponseManagers = Arc<Mutex<HashMap<usize, ResponseManager>>>;
type Cache = Arc<Mutex<HashMap<String, Vec<u8>>>>;

const MAX_CACHED_SIZE: usize = 20 * 1024 * 1024;


pub struct HosterManager {
    id: String,
    next_request_id: usize,
    mux: Multiplexer,
    response_managers: ResponseManagers,
    cache: Cache,
}

struct ResponseManager {
    cache_key: String,
    tx: oneshot::Sender<Response<Body>>,
}

impl HosterManager {
    pub fn new(id: String, ws: WebSocket, done_tx: mpsc::UnboundedSender<String>) -> Self {

        let cache = Arc::new(Mutex::new(HashMap::new()));
        let cache_clone = cache.clone();

        let transport = WebSocketTransport::new(ws);
        let mut mux = Multiplexer::new(transport);

        let rpc_set_id = json!({
            "jsonrpc": "2.0",
            "method": "setId",
            "params": id,
        }).to_string();

        mux.send_control_message(rpc_set_id.as_bytes().to_vec());

        let events = mux.events().expect("no events");

        let response_managers: ResponseManagers = Arc::new(Mutex::new(HashMap::new()));
        let response_managers_clone = response_managers.clone();

        let id_clone = id.clone();

        warp::spawn(events.for_each(move |event| {

            let id = (&id_clone).clone();

            match event {
                MultiplexerEvent::Close => {
                    println!("signal done");
                    done_tx.unbounded_send(id).unwrap();
                },
                MultiplexerEvent::ControlMessage(control_message) => {
                    let message: Value = serde_json::from_slice(&control_message)
                        .expect("parse control message");

                    println!("{}", message);

                    if message.get("error").is_some() {

                        match &message["id"] {
                            Value::Number(request_id) => {
                                let request_id = request_id.as_u64().expect("parse u64") as usize;
                                let mut lock = response_managers_clone.lock().expect("get lock");
                                let response_manager = lock.remove(&request_id).expect("removed tx");

                                let response = Response::builder()
                                    .status(404)
                                    .body(message["error"]["message"].to_string().into())
                                      .expect("error response");

                                response_manager.tx.send(response).unwrap();
                            },
                            _ => (),
                        }
                    }
                }
                MultiplexerEvent::Conduit(producer, metadata) => {

                    let md: Value = serde_json::from_slice(&metadata).expect("parse metadata");

                    println!("Create conduit");
                    println!("{}", md);

                    let request_id = md["id"].as_u64().expect("parse id") as usize;

                    let (stream_tx, stream_rx) = mpsc::channel::<Vec<u8>>(1);
                    let stream_rx = stream_rx.map_err(|_e| {
                        "stream fail"
                    });

                    // See if there's a way to do this without importing hyper
                    // directly.
                    let body = Body::wrap_stream(stream_rx);

                    let mut builder = Response::builder();

                    let size = md["result"]["size"].as_u64().expect("parse size") as usize;

                    match md["result"].get("range") {
                        Some(Value::Object(range)) => {
                            let start = range["start"].as_u64().expect("parse start") as usize;

                            let end = match range.get("end") {
                                Some(Value::Number(end)) => {
                                    end.as_u64().expect("parse end") as usize
                                },
                                _ => size,
                            };

                            let len = end - start;

                            // Need to subtract one from end because HTTP ranges are inclusive
                            let content_range = format!("bytes {}-{}/{}", start, end - 1, size);

                            builder
                                .status(206)
                                .header("Content-Range", content_range)
                                .header("Content-Length", len);
                        },
                        _ => {
                            builder.header("Content-Length", size);
                        },
                    }

                    let response = builder
                        .header("Accept-Ranges", "bytes")
                        .header("Content-Type", "application/octet-stream")
                        .body(body).expect("response");


                    let mut lock = response_managers_clone.lock().expect("get lock");
                    let response_manager = lock.remove(&request_id).expect("removed tx");
                    response_manager.tx.send(response).unwrap();

                    let cache = cache_clone.clone();
                    let cache_key = response_manager.cache_key.clone();

                    // TODO: this is hacky
                    let mut cached = if size <= MAX_CACHED_SIZE {
                        vec![0; size]
                    }
                    else {
                        Vec::new()
                    };

                    let mut index = 0;
                    let cache_conduit = MapConduit::new(move |data: Message| {

                        if size <= MAX_CACHED_SIZE {

                            for elem in &data {
                                cached[index] = *elem;
                                index += 1;
                            }

                            if index == size {
                                println!("add {} to cache", cache_key.clone());
                                cache.lock().expect("lock cache")
                                    // TODO: get rid of this extra clone
                                    .insert(cache_key.clone(), cached.clone());
                            }
                        }

                        data
                    });

                    let consumer = SinkAdapter::new(stream_tx);
                    producer
                        .pipe_through(cache_conduit)
                        .pipe_through(StatsConduit::new(request_id))
                        .pipe_into(consumer);
                }
            }
            Ok(())
        }));

        Self {
            id: id.clone(),
            next_request_id: 0,
            mux,
            response_managers,
            cache,
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

        let mut request = json!({
            "jsonrpc": "2.0",
            "method": "getFile",
            "params": json!({
                "path": format!("/{}", filename),
            }),
            "id": request_id,
        });

        let (response_tx, response_rx) = oneshot::channel();

        let range = parse_range_header(&range_header);

        if range.is_some() {
            request["params"]["range"] = range.unwrap();
        }
        else {
            match self.cache.lock().expect("lock cache").get(&filename) {
                Some(cached) => {
                    println!("serve {} from cache", filename);
                    // TODO: this early return is nastay
                    let response = Response::builder()
                        .body(cached.clone().into()).expect("error response");
                    response_tx.send(response).expect("response_tx send");
                    return response_rx;
                },
                None => (),
            }
        }

        self.mux.send_control_message(request.to_string().as_bytes().to_vec());

        let response_manager = ResponseManager {
            cache_key: filename,
            tx: response_tx,
        };
        self.response_managers.lock().expect("get lock").insert(request_id, response_manager);

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
            // Need to add one because HTTP ranges are inclusive
            let end = 1 + end_str.parse::<usize>().expect("parse range end");
            range["end"] = json!(end);
        }

        Some(range)
    }
}
