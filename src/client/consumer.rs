use rdkafka::{
    config::{ClientConfig, RDKafkaLogLevel},
    consumer::{stream_consumer::StreamConsumer, Consumer as RdKafkaConsumer},
};

pub(crate) struct Consumer {
    pub inner: StreamConsumer,
}

impl Consumer {
    /// Create a new Consumer.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```rust ignore
    /// let consumer = Consumer::new("localhost:9092", "my-unique-group", &["topic1"]);
    /// ```
    pub fn new(kafka_brokers: &str, group_id: &str, topics: &[&str]) -> Consumer {
        // Create the `Futureconsumer` to produce asynchronously.
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", kafka_brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set_log_level(RDKafkaLogLevel::Debug)
            .create()
            .expect("Consumer creation failed");

        consumer
            .subscribe(topics)
            .expect("Failed to subscribe to specified topics");

        Consumer { inner: consumer }
    }
}
