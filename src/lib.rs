pub mod errors;
mod schema;
pub use schema::FromFile;
mod client;
pub use client::{types, SchemaRegistryClient};
pub use serde_ext::SerdeExt;
mod serde_ext;
pub use apache_avro;
pub mod prelude {
    pub use super::schema::FromFile;
    pub use super::types::*;
    pub use super::SerdeExt;
}
