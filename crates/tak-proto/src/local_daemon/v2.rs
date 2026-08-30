//! Protocol v2 uses strict request shapes: additive fields require a new
//! protocol version rather than being silently ignored.

#[path = "v2/decoder.rs"]
mod decoder;
#[path = "v2/error_codes.rs"]
mod error_codes;
#[path = "v2/error_response.rs"]
mod error_response;
#[path = "v2/identifier.rs"]
mod identifier;
#[path = "v2/probe.rs"]
mod probe;
#[path = "v2/request_encoder.rs"]
mod request_encoder;
#[path = "v2/response.rs"]
mod response;
#[path = "v2/response_decoder.rs"]
mod response_decoder;
#[path = "v2/response_models.rs"]
mod response_models;
#[path = "v2/types.rs"]
mod types;
#[path = "v2/wire.rs"]
mod wire;

pub use decoder::decode_request;
pub use error_codes::{DaemonErrorCode, RequestEncodeError, ResponseDecodeError};
pub use error_response::ErrorResponse;
pub use request_encoder::encode_request;
pub use response::*;
pub use response_decoder::{
    MAX_ERROR_RESPONSE_FRAME_BYTES, MAX_RESPONSE_FRAME_BYTES, decode_error_response,
    decode_response,
};
pub use response_models::*;
pub use types::{DecodeOutcome, Operation, Request, RequestDecodeError, RequestDecodeErrorCode};

/// The only local daemon protocol accepted by v2 clients.
pub const PROTOCOL_VERSION: u64 = 2;
pub const MAX_REQUEST_FRAME_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WORKSPACE_CHUNK_BYTES: usize = 256 * 1024;
