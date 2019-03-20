use omnistreams::{
    Streamer, Producer, Consumer, Conduit, CancelReason, ConduitConsumer,
    ProducerEventRx, ConduitProducer, ConsumerEventRx, MapConduit, Message,
    pipe,
};

pub struct StatsConduit {
    consumer: ConduitConsumer<Message>,
    producer: ConduitProducer<Message>,
}

impl StatsConduit {
    pub fn new() -> Self {

        let mut total_items = 0;
        let mut total_bytes = 0;

        let item_counter = MapConduit::new(move |item: Message| {
            total_items += 1;
            println!("items: {}", total_items);
            item
        });

        let byte_counter = MapConduit::new(move |item: Message| {
            total_bytes += item.len();
            println!("bytes: {}", total_bytes);
            item
        });

        let (consumer, producer) = item_counter.split();
        let (c, p) = byte_counter.split();
        pipe(producer, c);

        Self {
            consumer,
            producer: p,
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
}

impl Producer<Message> for StatsConduit {
    fn request(&mut self, num_items: usize) {
        self.producer.request(num_items);
    }

    fn event_stream(&mut self) -> Option<ProducerEventRx<Message>> {
        self.producer.event_stream()
    }
}

impl Conduit<Message, Message> for StatsConduit {
    fn split(self) -> (ConduitConsumer<Message>, ConduitProducer<Message>) {
        (self.consumer, self.producer)
    }
}
