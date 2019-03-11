use warp::{self, Filter};
use warp::filters::ws::{Message, WebSocket};
use futures::{Future, Stream, Sink};
use futures::sync::mpsc;
use omnistreams::{Multiplexer, Transport};
use serde_json::json;
use uuid::Uuid;

type OmniMessage = Vec<u8>;
type MessageRx = mpsc::UnboundedReceiver<OmniMessage>;
type MessageTx = mpsc::UnboundedSender<OmniMessage>;

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
                println!("{:?}", omni_message);
                Message::binary(omni_message)
            })
            .forward(ws_sink)
            .map(|_| ());

        warp::spawn(out_task);


        let in_tx = in_tx.sink_map_err(|_e| ());

        let in_task = ws_stream.map_err(|_e| ())
            .map(|ws_message| {
                println!("ws_message: {:?}", ws_message);
                vec![0, 0, 65]
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
        println!("sendy");
        self.out_tx.unbounded_send(message).expect("ws transport send");
    }
    fn messages(&mut self) -> Option<mpsc::UnboundedReceiver<OmniMessage>> {
        Option::take(&mut self.in_rx)
    }
}

fn main() {
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

                //mux.sendControlMessage(encodeObject({
                //  type: 'complete-handshake',
                //  id,
                //}))

                //let (tx, rx) = socket.split();

                //let f = tx 
                //    .send(Message::text("Hi there"))
                //    .map(|_| ())
                //    .map_err(|_| ());

                ////let x: i32 = f;

                //warp::spawn(f);

                //rx.map_err(|_| {})
                //.for_each(|msg| {
                //    println!("{:?}", msg);
                //    Ok(())
                //})
                futures::future::ok(())
            })
        });

    let index = warp::path::end().map(|| {
        warp::reply::html("<h1>Hi there</h1>")
    });

    let routes = index.or(omnis);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 9001));
}
