use crate::client::types::*;
use crate::client::ResponseExt;
use crate::errors::SRError;
use async_lock::Mutex;
use avro_rs::{from_value, Reader, Schema};
use isahc::{
    auth::{Authentication, Credentials},
    config::{RedirectPolicy, VersionNegotiation},
    prelude::*,
    HttpClient,
};
use isahc::{AsyncBody, AsyncReadResponseExt, ResponseFuture};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Create an instance of SchemaRegistryClient
///
/// ```rust,no_run
/// use schema_registry_client::prelude::*;
/// use schema_registry_client::SchemaRegistryClient;
///
/// let client = SchemaRegistryClient::new("http://localhost:8081",
///         Auth::Basic{
///         username: "username".to_string(),
///         password: "password".to_string(),
///     })
///     .unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct SchemaRegistryClient {
    httpclient: isahc::HttpClient,
    url: String,
    cache: Arc<Mutex<HashMap<i32, String>>>,
}

impl SchemaRegistryClient {
    /// Create a new instance of SchemaRegistryClient
    ///
    /// ```rust,no_run
    /// use serde_json::Value;
    /// use schema_registry_client::SchemaRegistryClient;
    /// use schema_registry_client::types::Auth;
    ///
    /// # #[tokio::main]
    /// # async fn main()-> Result<(), Box<dyn std::error::Error>>{
    /// let client = SchemaRegistryClient::new("http://localhost:8081", Auth::None);
    /// //... use client here
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(url: &str, auth: Auth) -> Result<Self, SRError> {
        let builder = HttpClient::builder()
            .version_negotiation(VersionNegotiation::http11())
            .redirect_policy(RedirectPolicy::Limit(10))
            .timeout(Duration::from_secs(20))
            .default_header("Content-Type", "application/vnd.schemaregistry.v1+json");

        let httpclient = match auth {
            Auth::Basic { username, password } => builder
                .authentication(Authentication::basic())
                .credentials(Credentials::new(username, password)),
            Auth::None => builder,
        }
        .build()?;

        Ok(Self {
            httpclient,
            url: url.into(),
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn get_subject_from_topic(&self, topic: &str, subject: SchemaSubjectType) -> String {
        match subject {
            SchemaSubjectType::Key => format!("{}-key", topic),
            SchemaSubjectType::Value => format!("{}-value", topic),
        }
    }

    pub async fn request<T, R>(
        &self,
        url: &str,
        method: isahc::http::Method,
        body: T,
    ) -> Result<R, SRError>
    where
        R: serde::de::DeserializeOwned + std::marker::Unpin,
        T: Into<AsyncBody>,
    {
        tracing::debug!("URL is {:?}", &url);
        let rfuture: ResponseFuture = match method {
            isahc::http::Method::GET => self.httpclient.get_async(url),
            isahc::http::Method::PUT => self.httpclient.put_async(url, body),
            isahc::http::Method::POST => self.httpclient.post_async(url, body),
            isahc::http::Method::DELETE => self.httpclient.delete_async(url),
            _ => return Err(SRError::UnsupportedHTTPMethod(method.to_string())),
        };

        let json_response = rfuture
            .await?
            .check_for_error()
            .await?
            .json::<R>()
            .await
            .map_err(|source| SRError::Serde { source })?;
        Ok(json_response)
    }

    /// Register the given schema to schema-registry
    pub async fn register_schema(
        &self,
        schema: &Schema,
        topic: &str,
        subject_type: SchemaSubjectType,
    ) -> Result<SchemaRegistrationResponse, SRError> {
        let url = format!(
            "{url}/subjects/{subject}/versions",
            url = self.url,
            subject = self.get_subject_from_topic(topic, subject_type),
        );

        let payload = serde_json::json!({
            "schema": schema.canonical_form()
        });
        let body = serde_json::to_vec(&payload)?;
        let resp = self.request(&url, isahc::http::Method::POST, body).await?;
        Ok(resp)
    }

    /// Get the schema
    pub async fn get_schema(&self, id: i32) -> Result<String, SRError> {
        let mut c = self.cache.lock().await;
        let schema = match c.get(&id) {
            Some(sc) => {
                tracing::debug!("Found in cache.");
                sc.to_string()
            }
            None => {
                tracing::debug!("Not in cache, making a schema registry call.");
                let url = format!("{url}/schemas/ids/{id}", url = self.url, id = id,);
                let resp: SchemaGetResponse =
                    self.request(&url, isahc::http::Method::GET, ()).await?;
                c.insert(id, resp.schema.clone());
                resp.schema
            }
        };
        Ok(schema)
    }

    /// Get the latest schema
    pub async fn get_schema_latest(&self) -> Result<String, SRError> {
        let url = format!("{url}/schemas/ids/latest", url = self.url);
        let resp: SchemaGetResponse = self.request(&url, isahc::http::Method::GET, ()).await?;
        Ok(resp.schema)
    }
}

// pub async fn deserialize_message<T: serde::de::DeserializeOwned + Clone>(
//     client: &SchemaRegistryClient,
//     msg: &[u8],
// ) -> Result<T, SRError> {
//     let mut buf = &msg[1..5];
//     let id = buf.read_i32::<BigEndian>().unwrap();
//     println!("{:?}", id);
//     let schema = client.get_schema(id).await?;
//     let schema = Schema::parse_str(&schema)?;

//     let payload = &msg[5..];
//     let result = Reader::with_schema(&schema, payload)?
//         .into_iter()
//         .map(|val| from_value::<T>(&val?))
//         .take(1)
//         .collect::<Result<Vec<T>, _>>()?;
//     Ok(result[0].clone())
// }

pub async fn deserialize_message<T: serde::de::DeserializeOwned + Clone>(
    client: &SchemaRegistryClient,
    msg: &[u8],
) -> Result<T, SRError> {
    // TODO (ansrivas): Check if the msg len is > 5 and magic byte is set

    let mut buf = [0u8; 4];
    buf.copy_from_slice(&msg[1..5]);
    let id = i32::from_be_bytes(buf);

    let schema = client.get_schema(id).await?;
    let schema = Schema::parse_str(&schema)?;

    let payload = &msg[5..];
    let result = Reader::with_schema(&schema, payload)?
        .into_iter()
        .map(|val| from_value::<T>(&val?))
        .take(1)
        .collect::<Result<Vec<T>, _>>()?;
    Ok(result[0].clone())
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::client::admin::AdminClient;
    use crate::client::consumer::Consumer;
    use crate::client::producer::Producer;

    use crate::FromFile;
    use avro_rs::Schema;
    use futures_lite::StreamExt;
    use rdkafka::Message;
    use serde::{Deserialize, Serialize};
    use tokio::sync::oneshot;
    use tokio::time::{timeout_at, Instant};

    fn test_client() -> SchemaRegistryClient {
        let url = "http://localhost:8081";
        let auth = Auth::None;
        SchemaRegistryClient::new(url, auth).unwrap()
    }

    fn test_schema() -> Schema {
        Schema::parse_file("tests/data/schema2.avsc").unwrap()
    }

    /// Generate a random length string
    pub fn random_chars(length: usize, prefix: &str) -> String {
        use rand::{distributions::Alphanumeric, Rng};

        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect();
        format!("{}{}", prefix, suffix)
    }

    #[tokio::test]
    async fn test_register_and_get_schema() {
        let client = test_client();
        let topic = random_chars(10, "test");
        let schema = test_schema();
        let res = client
            .register_schema(&schema, &topic, SchemaSubjectType::Value)
            .await
            .unwrap();
        assert!(res.id > 0);

        let resp = client.get_schema(1).await.unwrap();
        assert!(Schema::parse_str(&resp).unwrap() == test_schema());
        assert!(!resp.is_empty())
    }

    #[tokio::test]
    async fn test_producer_consumer() {
        let bootstrap_server = "localhost:9092";
        // let topic = random_chars(5, "topic_");
        let topic = "topic";

        let admin_client = AdminClient::new(bootstrap_server);
        admin_client.create_topic(&topic).await;

        let sr_client = test_client();

        let schema = test_schema();
        let res = sr_client
            .register_schema(&schema, &topic, SchemaSubjectType::Value)
            .await
            .unwrap();

        let (tx, rx) = oneshot::channel::<Vec<u8>>();

        #[derive(Serialize, Deserialize, Clone, Debug)]
        struct MyRecord {
            pub f1: String,
            pub f2: String,
        }

        let record = MyRecord {
            f1: "ankur".to_string(),
            f2: "srivastava".to_string(),
        };

        let group_id = random_chars(5, "group_");
        let consumer = Consumer::new(bootstrap_server, &group_id, &[&topic]);
        tokio::spawn(async move {
            tracing::info!("**** Spawned the agent ****");
            let mut stream = consumer.inner.stream();
            while let Some(msg) = stream.next().await {
                tx.send(msg.unwrap().payload().unwrap().to_vec()).unwrap();
                break;
            }
        });

        // Giving sometime for this consumer to be spawned
        tokio::time::sleep(Duration::from_secs(1)).await;

        println!("schema-id: {}", &res.id);

        let prod = Producer::new_with_schema_registry(bootstrap_server, None, None, sr_client);
        prod.produce_avro_with_schema(&record, &topic, res.id, Some("key"))
            .await
            .unwrap();

        match timeout_at(Instant::now() + Duration::from_secs(5), rx).await {
            Ok(Ok(v)) => {
                let record: MyRecord = deserialize_message(&test_client(), &v).await.unwrap();
                println!("Record is {:?}", &record);
            }
            Ok(Err(e)) => assert!(false, "{:?}", e),
            Err(e) => assert!(false, "Timeout occurred: {:?}", e),
        }
    }
}
