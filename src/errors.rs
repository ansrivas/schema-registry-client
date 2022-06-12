// MIT License
//
// Copyright (c) 2022 Ankur Srivastava
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Errors to be used with the library, converts to and from
//! other dependencies' errors.

use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum SRError {
    #[error("File read error")]
    IO(#[from] std::io::Error),

    #[error("Schema parsing error")]
    Schema(#[from] avro_rs::Error),

    #[error("Serializing/Deserializing error  {source}")]
    Serde {
        #[from]
        source: serde_json::Error,
    },

    #[error("Status Code `{error_code}` Message: {message}")]
    SrHttp { error_code: i32, message: String },

    #[error("{0}")]
    Custom(String),

    #[error("HTTPClientError: {source}")]
    HTTPClient {
        #[from]
        source: isahc::Error,
    },

    #[error("HTTPRequestError: {0}")]
    HTTPRequestError(String),

    #[error("HTTPError: {source}")]
    Http {
        #[from]
        source: isahc::http::Error,
    },

    #[error("UnsupportedHTTPMethod: {0}")]
    UnsupportedHTTPMethod(String),

    // Kafka send/receive error
    #[error(transparent)]
    KafkaError(#[from] rdkafka::error::KafkaError),
}
