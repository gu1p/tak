use anyhow::Result;
use tak_proto::{decode_remote_token, decode_tor_invite};

use super::provider::GrayFrame;

#[derive(Clone)]
pub(super) struct ScanMatch {
    pub(super) token: String,
    pub(super) node_id: String,
    pub(super) display_name: String,
    pub(super) base_url: String,
    pub(super) transport: String,
}

pub(super) fn decode_frame(frame: &GrayFrame) -> Result<Option<ScanMatch>> {
    let mut decoder = quircs::Quirc::default();
    for candidate in decoder.identify(frame.width as usize, frame.height as usize, &frame.pixels) {
        let Ok(code) = candidate else { continue };
        let Ok(decoded) = code.decode() else { continue };
        let Ok(payload) = String::from_utf8(decoded.payload) else {
            continue;
        };
        if let Ok(token) = decode_remote_token(&payload) {
            let Some(node) = token.node else { continue };
            if node.transport == "tor" {
                continue;
            }
            return Ok(Some(ScanMatch {
                token: payload,
                node_id: node.node_id,
                display_name: node.display_name,
                base_url: node.base_url,
                transport: node.transport,
            }));
        }
        let Ok(base_url) = decode_tor_invite(&payload) else {
            continue;
        };
        return Ok(Some(ScanMatch {
            token: payload,
            node_id: "(discover on add)".into(),
            display_name: "(discover on add)".into(),
            base_url,
            transport: "tor".into(),
        }));
    }
    Ok(None)
}
