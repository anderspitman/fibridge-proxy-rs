use omnistreams::{
    Streamer, Producer, Consumer, Conduit, CancelReason,
    ProducerEventRx, ConsumerEventRx, MapConduit, Message,
    MapConsumer, MapProducer,
};

pub struct StatsConduit {
    consumer: MapConsumer<Message>,
    producer: MapProducer<Message>,
}

impl StatsConduit {
    pub fn new(_request_id: usize) -> Self {

        let mut total_items = 0;
        let mut total_bytes = 0;

        let item_counter = MapConduit::new(move |item: Message| {
            total_items += 1;
            //println!("{} items: {}", request_id, total_items);
            item
        });

        let byte_counter = MapConduit::new(move |item: Message| {
            total_bytes += item.len();
            //println!("{} bytes: {}", request_id, total_bytes);
            item
        });

        let (c_item, p_item) = item_counter.split();
        let (c_byte, p_byte) = byte_counter.split();
        p_item.pipe_into(c_byte);

        Self {
            consumer: c_item,
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

    type ConcreteConsumer = MapConsumer<Message>;
    type ConcreteProducer = MapProducer<Message>;

    fn split(self) -> (Self::ConcreteConsumer, Self::ConcreteProducer) {
        (self.consumer, self.producer)
    }
}
