use futures::sync::mpsc;
use futures::{Future, Stream, Sink};
use warp::filters::ws::{Message, WebSocket};
use omnistreams::{Transport, TransportEvent, TransportEventRx};

type OmniMessage = Vec<u8>;
type MessageTx = mpsc::UnboundedSender<OmniMessage>;


pub struct WebSocketTransport {
    out_tx: MessageTx,
    event_rx: Option<TransportEventRx>,
}

impl WebSocketTransport {
    pub fn new(ws: WebSocket) -> Self {
        let (ws_sink, ws_stream) = ws.split();
        let (out_tx, out_rx) = mpsc::unbounded::<OmniMessage>();
        let (event_tx, event_rx) = mpsc::unbounded::<TransportEvent>();

        let ws_sink = ws_sink
            .sink_map_err(|_e| ());

        let out_task = out_rx.map_err(|_e| ())
            .map(|omni_message| {
                Message::binary(omni_message)
            })
            .forward(ws_sink)
            .map(|_| ());

        warp::spawn(out_task);


        let event_tx = event_tx.sink_map_err(|_e| {
            eprintln!("transport sink_map_err");
        });

        let in_task = ws_stream.map_err(|_e| ())
            .map(|ws_message| {
                TransportEvent::Message(ws_message.as_bytes().to_vec())
            })
            .forward(event_tx)
            .map(|(_, sink)| {
                //let sink = sink.sink_map_err(|_| ());
                //warp::spawn(sink.send(TransportEvent::Close).map(|_| ()));
            });

        warp::spawn(in_task);


        Self {
            out_tx,
            event_rx: Some(event_rx),
        }
    }
}

impl Transport for WebSocketTransport {

    fn send(&mut self, message: OmniMessage) {
        self.out_tx.unbounded_send(message).expect("ws transport send");
    }

    fn event_stream(&mut self) -> Option<TransportEventRx> {
        Option::take(&mut self.event_rx)
    }
}
