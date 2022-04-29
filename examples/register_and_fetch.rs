use dashmap::DashMap;
use schema_registry_client::{avro_rs::Schema, types::Auth, SchemaRegistryClient};
use std::sync::Arc;

async fn query_endpoint(id: i32) -> Schema {
    let auth = Auth::None;
    let client = SchemaRegistryClient::new("http://localhost:8081", auth).unwrap();
    let response = client.get_schema(id).await.unwrap();
    Schema::parse_str(&response.schema).unwrap()
}

async fn get_or_cache(cache: Arc<DashMap<i32, Schema>>, id: i32) -> Schema {
    if let Some(schema) = cache.get(&id) {
        println!("Returning from cache");
        schema.value().clone()
    } else {
        println!("Returning from schema-registry");
        let schema = query_endpoint(id).await;
        cache.insert(id, schema.clone());
        schema
    }
}

#[tokio::main]
async fn main() {
    let cache = Arc::new(DashMap::new());
    let _schema = get_or_cache(cache.clone(), 1).await;
    let _schema = get_or_cache(cache, 1).await;
}
