//! Protocol v2 uses strict request shapes: additive fields require a new
//! protocol version rather than being silently ignored.

#[path = "v2/decoder.rs"]
mod decoder;
#[path = "v2/error_response.rs"]
mod error_response;
#[path = "v2/identifier.rs"]
mod identifier;
#[path = "v2/probe.rs"]
mod probe;
#[path = "v2/types.rs"]
mod types;
#[path = "v2/wire.rs"]
mod wire;

pub use decoder::decode_request;
pub use error_response::ErrorResponse;
pub use types::{DecodeOutcome, Operation, Request, RequestDecodeError, RequestDecodeErrorCode};

/// The only local daemon protocol accepted by v2 clients.
pub const PROTOCOL_VERSION: u64 = 2;
