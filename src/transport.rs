use futures::sync::mpsc;
use futures::{Future, Stream, Sink};
use warp::filters::ws::{Message, WebSocket};
use omnistreams::{Transport};

type OmniMessage = Vec<u8>;
type MessageRx = mpsc::UnboundedReceiver<OmniMessage>;
type MessageTx = mpsc::UnboundedSender<OmniMessage>;


pub struct WebSocketTransport {
    out_tx: MessageTx,
    in_rx: Option<MessageRx>,
}

impl WebSocketTransport {
    pub fn new(ws: WebSocket) -> Self {
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
