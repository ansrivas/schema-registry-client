use schema_registry_client::{types::Auth, SchemaRegistryClient};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{self, fmt, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let id = 1;
    let auth = Auth::None;
    let client = SchemaRegistryClient::new("http://localhost:8081", auth).unwrap();
    let _schema = client.get_schema(id).await.unwrap();
    let _schema = client.get_schema(id).await.unwrap();
}
