use serde::Deserializer as _;
use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde_json::value::RawValue;

use super::identifier::is_valid_identifier;
use super::{PROTOCOL_VERSION, RequestDecodeErrorCode};

#[derive(Debug, Default)]
pub(super) struct Probe {
    version_count: usize,
    version: Version,
    request_id_count: usize,
    request_id: Option<String>,
    operation_seen: bool,
    complete: bool,
}

#[derive(Debug, Default)]
enum Version {
    #[default]
    Missing,
    Invalid,
    Exact,
    Unsupported,
}

pub(super) enum Decision {
    StrictV2,
    Reject(RequestDecodeErrorCode),
}

impl Probe {
    pub(super) fn inspect(raw: &str) -> Self {
        let mut probe = Self::default();
        let mut deserializer = serde_json::Deserializer::from_str(raw);
        let parsed = deserializer.deserialize_map(ProbeVisitor(&mut probe));
        probe.complete = parsed.is_ok() && deserializer.end().is_ok();
        probe
    }

    pub(super) fn decide(&self) -> Decision {
        if !self.complete {
            return if self.version_count > 0 || self.operation_seen {
                Decision::Reject(RequestDecodeErrorCode::RequestInvalid)
            } else {
                Decision::Reject(RequestDecodeErrorCode::VersionUnsupported)
            };
        }
        if self.version_count == 0 {
            return if self.operation_seen {
                Decision::Reject(RequestDecodeErrorCode::VersionInvalid)
            } else {
                Decision::Reject(RequestDecodeErrorCode::VersionUnsupported)
            };
        }
        if self.version_count > 1 || matches!(self.version, Version::Invalid) {
            return Decision::Reject(RequestDecodeErrorCode::VersionInvalid);
        }
        match self.version {
            Version::Exact => Decision::StrictV2,
            Version::Unsupported => Decision::Reject(RequestDecodeErrorCode::VersionUnsupported),
            Version::Missing | Version::Invalid => {
                Decision::Reject(RequestDecodeErrorCode::VersionInvalid)
            }
        }
    }

    pub(super) fn request_id(&self) -> Option<String> {
        if self.complete && self.request_id_count == 1 {
            return self.request_id.clone();
        }
        None
    }

    fn begin_version(&mut self) {
        self.version_count = self.version_count.saturating_add(1);
        self.version = Version::Invalid;
    }

    fn finish_version(&mut self, raw: &RawValue) {
        if self.version_count == 1 {
            self.version = classify_version(raw.get());
        }
    }

    fn begin_request_id(&mut self) {
        self.request_id_count = self.request_id_count.saturating_add(1);
        self.request_id = None;
    }

    fn finish_request_id(&mut self, raw: &RawValue) {
        if self.request_id_count != 1 || !raw.get().starts_with('"') {
            return;
        }
        self.request_id = serde_json::from_str::<String>(raw.get())
            .ok()
            .filter(|value| is_valid_identifier(value));
    }
}

fn classify_version(raw: &str) -> Version {
    let value = raw.trim();
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Version::Invalid;
    }
    match value.parse::<u64>() {
        Ok(PROTOCOL_VERSION) => Version::Exact,
        Ok(_) => Version::Unsupported,
        Err(_) => Version::Invalid,
    }
}

struct ProbeVisitor<'a>(&'a mut Probe);

impl<'de> Visitor<'de> for ProbeVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a local daemon request object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "protocol_version" => {
                    self.0.begin_version();
                    let raw = map.next_value::<&RawValue>()?;
                    self.0.finish_version(raw);
                }
                "request_id" => {
                    self.0.begin_request_id();
                    let raw = map.next_value::<&RawValue>()?;
                    self.0.finish_request_id(raw);
                }
                "operation" => {
                    self.0.operation_seen = true;
                    map.next_value::<IgnoredAny>()?;
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        Ok(())
    }
}
