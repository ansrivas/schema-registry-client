use crate::errors::SRError;
use crate::SchemaRegistryClient;
use avro_rs::{Schema, Writer};
use rdkafka::producer::DeliveryFuture;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};

use serde::Serialize;
use tracing::instrument;

#[derive(Clone)]
pub struct Producer {
    producer: FutureProducer,
    sr_client: Option<SchemaRegistryClient>,
}

impl Producer {
    /// Create a new Producer instance with a provided FutureProducer
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```rust,no_run
    /// use privacy_service::kafka::Producer;
    /// let kproducer = Producer::new("localhost:9092", Some("user"), Some("password"));
    /// ```
    pub fn new(brokers: &str, username: Option<&str>, password: Option<&str>) -> Producer {
        let mut config: ClientConfig = Self::default_config(brokers);
        let producer: FutureProducer = match (username, password) {
            (Some(user), Some(pass)) => config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.username", user)
                .set("sasl.password", pass)
                .set("sasl.mechanisms", "PLAIN")
                .create()
                .expect("Producer creation error"),
            _ => config.create().expect("Producer creation error"),
        };
        Producer {
            producer,
            sr_client: None,
        }
    }

    pub fn new_with_schema_registry(
        brokers: &str,
        username: Option<&str>,
        password: Option<&str>,
        sr_client: SchemaRegistryClient,
    ) -> Self {
        let mut config: ClientConfig = Self::default_config(brokers);
        let producer: FutureProducer = match (username, password) {
            (Some(user), Some(pass)) => config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.username", user)
                .set("sasl.password", pass)
                .set("sasl.mechanisms", "PLAIN")
                .create()
                .expect("Producer creation error"),
            _ => config.create().expect("Producer creation error"),
        };

        Producer {
            producer,
            sr_client: Some(sr_client),
        }
    }

    /// Bunch of default configuration setup to create a client
    fn default_config(brokers: &str) -> ClientConfig {
        vec![
            ("bootstrap.servers".into(), brokers.into()),
            ("produce.offset.report".into(), "true".into()),
            ("message.timeout.ms".into(), "60000".into()),
            ("queue.buffering.max.messages".into(), "10".into()),
            ("acks".into(), "all".into()),
        ]
        .into_iter()
        .collect()
    }

    #[instrument(name = "produce_kafka_msg", err, level = "info", skip(self, payload))]
    async fn produce(
        &self,
        topic: &str,
        key: Option<&str>,
        payload: Vec<u8>,
    ) -> Result<DeliveryFuture, SRError> {
        let record = FutureRecord::to(topic)
            .key(key.unwrap_or_default())
            .payload(&payload[..]);
        self.producer.send_result(record).map_err(|e| e.0.into())
    }

    /// Publish a BytesMut record to a given topic on Kafka.
    #[instrument(level = "info", err(Debug), skip(self, data))]
    pub async fn produce_json<T: ?Sized + Serialize>(
        &self,
        data: &T,
        topic: &str,
        key: Option<&str>,
    ) -> Result<DeliveryFuture, SRError> {
        let payload = serde_json::to_vec(data).unwrap();
        self.produce(topic, key, payload).await
    }

    /// Publish a record to a given topic on Kafka.
    /// If the producer was created with SchemaRegistry client
    /// it will use schemas to encode the values.
    /// NOTE: If you passed a schema_id here, its assumed that
    /// you also defined the schema-registry client.
    #[instrument(level = "info", err, skip(self, data))]
    async fn produce_avro_inner<T: ?Sized + Serialize>(
        &self,
        data: &T,
        topic: &str,
        schema_id: Option<i32>,
        key: Option<&str>,
    ) -> Result<DeliveryFuture, SRError> {
        let payload = if let Some(id) = schema_id {
            let schema_str = self.sr_client.as_ref().unwrap().get_schema(id).await?;
            tracing::debug!("Schema received from SR: {}", &schema_str);

            let schema = Schema::parse_str(&schema_str)?;
            let mut writer = Writer::new(&schema, Vec::new());
            writer.append_ser(data)?;
            let pp = writer.into_inner()?; // This flushes the buffer

            let magic_byte = 0u8;
            let mut payload = vec![magic_byte];
            let header = id.to_be_bytes().to_vec();
            payload.extend_from_slice(&header);
            payload.extend_from_slice(&pp);
            payload
        } else {
            serde_json::to_vec(data).unwrap()
        };
        self.produce(topic, key, payload).await
    }

    #[instrument(level = "info", err, skip(self, data))]
    pub async fn produce_avro<T: ?Sized + Serialize>(
        &self,
        data: &T,
        topic: &str,
        key: Option<&str>,
    ) -> Result<DeliveryFuture, SRError> {
        self.produce_avro_inner(data, topic, None, key).await
    }

    #[instrument(level = "info", err, skip(self, data))]
    pub async fn produce_avro_with_schema<T: ?Sized + Serialize>(
        &self,
        data: &T,
        topic: &str,
        schema_id: i32,
        key: Option<&str>,
    ) -> Result<DeliveryFuture, SRError> {
        self.produce_avro_inner(data, topic, Some(schema_id), key)
            .await
    }
}

// #[cfg(test)]
// mod tests {

//     use crate::kafka::AdminClient;
//     use crate::kafka::Consumer;
//     use crate::kafka::Producer;
//     use crate::utils;
//     use futures_lite::StreamExt;
//     use rdkafka::Message;
//     use std::time::Duration;
//     use tokio::sync::oneshot;
//     use tokio::time::{timeout_at, Instant};

//     #[tokio::test]
//     async fn test_producer_consumer() {
//         use crate::monitoring::observability;
//         observability::setup_tracing("test-app", None);

//         #[derive(serde::Serialize, serde::Deserialize)]
//         struct Customer {
//             customer_id: i32,
//         }
//         let bootstrap_server = "localhost:9092";
//         let topic = utils::random_chars(5, "topic_");

//         let admin_client = AdminClient::new(bootstrap_server);
//         admin_client.create_topic(&topic).await;

//         let (tx, rx) = oneshot::channel::<Vec<u8>>();

//         let group_id = utils::random_chars(5, "group_");
//         let consumer = Consumer::new(bootstrap_server, &group_id, &[&topic]);
//         tokio::spawn(async move {
//             tracing::info!("**** Spawned the agent ****");
//             let mut stream = consumer.inner.stream();
//             while let Some(msg) = stream.next().await {
//                 tx.send(msg.unwrap().payload().unwrap().to_vec()).unwrap();
//                 break;
//             }
//         });

//         let producer = Producer::new(bootstrap_server, None, None);
//         let customer_id = utils::random_ints(5);
//         let payload = Customer {
//             customer_id: customer_id,
//         };

//         // Giving sometime for this consumer to be spawned
//         tokio::time::sleep(Duration::from_secs(5)).await;

//         producer
//             .produce_json(&payload, &topic, Some(&customer_id.to_string()))
//             .await
//             .unwrap();

//         match timeout_at(Instant::now() + Duration::from_secs(5), rx).await {
//             Ok(Ok(v)) => {
//                 let resp: Customer = serde_json::from_slice(&v).unwrap();
//                 assert!(resp.customer_id == customer_id)
//             }
//             Ok(Err(e)) => assert!(false, "{:?}", e),
//             Err(e) => assert!(false, "Timeout occurred: {:?}", e),
//         }
//     }
// }
