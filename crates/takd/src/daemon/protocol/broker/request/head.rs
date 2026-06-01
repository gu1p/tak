pub(in crate::daemon::protocol::broker) struct LocalBrokerRequestHead {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    content_length: usize,
}

impl LocalBrokerRequestHead {
    pub(super) fn new(
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        content_length: usize,
    ) -> Self {
        Self {
            method,
            path,
            headers,
            content_length,
        }
    }

    pub(in crate::daemon::protocol::broker) fn method(&self) -> &str {
        &self.method
    }

    pub(in crate::daemon::protocol::broker) fn path(&self) -> &str {
        &self.path
    }

    pub(in crate::daemon::protocol::broker) fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub(in crate::daemon::protocol::broker) fn content_length(&self) -> usize {
        self.content_length
    }

    pub(in crate::daemon::protocol::broker) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}
