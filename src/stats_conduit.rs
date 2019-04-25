use std::sync::{Arc, Mutex};
use futures::{Future, Stream, Sink};
use futures::sync::{mpsc};
use omnistreams::{
    Streamer, Producer, Consumer, Conduit, CancelReason,
    ProducerEventRx, ConsumerEventRx, MapConduit, Message,
    MapConsumer, MapProducer, ConsumerEvent,
};

pub struct StatsConduit {
    consumer: MapConsumer<Message>,
    producer: MapProducer<Message>,
}

impl StatsConduit {
    pub fn new(request_id: usize) -> Self {

        let total_bytes: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let total_bytes_clone = total_bytes.clone();

        let byte_counter = MapConduit::new(move |item: Message| {
            let mut val = total_bytes.lock().expect("lock");
            *val += item.len();
            item
        });


        let (mut c_byte, p_byte) = byte_counter.split();

        let events = c_byte.event_stream().expect("get events");

        let (tx, rx) = mpsc::unbounded();
        let tx = tx.sink_map_err(|_| ());

        let events_fut = events.map(move |event| {
            match event {
                ConsumerEvent::Cancellation(_) => {
                    let val = total_bytes_clone.lock().expect("lock");
                    println!("Canceled {} after {} bytes", request_id, *val);
                },
                _ => (),
            }

            event
        })
        .forward(tx)
        .map(|_| ());

        warp::spawn(events_fut);

        c_byte.set_event_stream(rx);

        Self {
            consumer: c_byte,
            producer: p_byte,
        }
    }
}

impl Streamer for StatsConduit {
    fn cancel(&mut self, _reason: CancelReason) {
        // TODO: implement cancel
    }
}

impl Consumer<Message> for StatsConduit {
    fn write(&self, data: Message) {
        self.consumer.write(data);
    }

    fn end(&self) {
        self.consumer.end();
    }

    fn event_stream(&mut self) -> Option<ConsumerEventRx> {
        self.consumer.event_stream()
    }

    fn set_event_stream(&mut self, event_stream: ConsumerEventRx) {
        self.consumer.set_event_stream(event_stream);
    }
}

impl Producer<Message> for StatsConduit {
    fn request(&mut self, num_items: usize) {
        self.producer.request(num_items);
    }

    fn event_stream(&mut self) -> Option<ProducerEventRx<Message>> {
        self.producer.event_stream()
    }

    fn set_event_stream(&mut self, event_stream: ProducerEventRx<Message>) {
        self.producer.set_event_stream(event_stream);
    }
}

impl Conduit<Message, Message> for StatsConduit {

    type ConcreteConsumer = MapConsumer<Message>;
    type ConcreteProducer = MapProducer<Message>;

    fn split(self) -> (Self::ConcreteConsumer, Self::ConcreteProducer) {
        (self.consumer, self.producer)
    }
}
