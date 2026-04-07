mod generated;
mod token;

pub use generated::tak::proto::v1::*;
pub use token::{decode_remote_token, encode_remote_token};
