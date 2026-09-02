use super::*;

mod http2;
mod legacy_http;
mod response;
mod tor_client;

use http2::{BrokerHttp2Request, BrokerHttp2Response};
use response::BrokerHttpError;
use tor_client::BrokerBody;
pub use tor_client::{BrokerForwardResponse, TorBroker};

const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 512 * 1024 * 1024;
