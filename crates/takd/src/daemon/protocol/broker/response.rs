#[derive(Clone)]
pub(super) struct BrokerHttpError {
    code: &'static str,
    detail: String,
}

impl BrokerHttpError {
    pub(super) fn bad_request_with_source(
        code: &'static str,
        source: impl std::fmt::Display,
    ) -> Self {
        Self {
            code,
            detail: format!("{code}: {source}"),
        }
    }

    pub(super) fn bad_gateway(code: &'static str, source: impl std::fmt::Display) -> Self {
        Self {
            code,
            detail: format!("{code}: {source}"),
        }
    }

    pub(super) fn code(&self) -> &'static str {
        self.code
    }
}

impl From<BrokerHttpError> for anyhow::Error {
    fn from(value: BrokerHttpError) -> Self {
        anyhow::anyhow!(value.detail)
    }
}
