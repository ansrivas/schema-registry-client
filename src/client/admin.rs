use rdkafka::admin::{AdminClient as RdKafkaAdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::ClientConfig;
use std::time::Duration;

pub struct AdminClient {
    client: RdKafkaAdminClient<DefaultClientContext>,
}

impl AdminClient {
    pub fn new(bootstrap_server: &str) -> Self {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", bootstrap_server);
        let admin_client: RdKafkaAdminClient<DefaultClientContext> =
            config.create().expect("admin client creation failed");

        AdminClient {
            client: admin_client,
        }
    }

    /// Useful  for testing purposes.
    pub async fn create_topic(&self, topic: &str) {
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(1)));

        let topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        self.client
            .create_topics(&[topic], &opts)
            .await
            .expect("topic creation failed");
    }
}

// #[cfg(test)]
// mod tests {

// 	use super::*;
// 	use crate::utils;
// 	use rdkafka::admin::{OwnedResourceSpecifier, ResourceSpecifier};

// 	#[tokio::test]
// 	async fn test_topic_create() {
// 		let adm_client = AdminClient::new("localhost:9092");

// 		let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(1)));

// 		let topic_name = utils::random_chars_without_prefix(5);
// 		adm_client.create_topic(&topic_name).await;

// 		let description = adm_client
// 			.client
// 			.describe_configs(&[ResourceSpecifier::Topic(&topic_name)], &opts)
// 			.await
// 			.unwrap();
// 		for d in description.iter() {
// 			assert!(
// 				d.as_ref().unwrap().specifier == OwnedResourceSpecifier::Topic(topic_name.clone())
// 			);
// 		}
// 	}
// }
