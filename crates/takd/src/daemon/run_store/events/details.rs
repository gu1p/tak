pub(in crate::daemon::run_store) struct JobEventDetails<'a> {
    pub(in crate::daemon::run_store) message: &'a str,
    pub(in crate::daemon::run_store) authored_attempt: Option<u32>,
}

impl<'a> JobEventDetails<'a> {
    pub(in crate::daemon::run_store) fn new(message: &'a str) -> Self {
        Self {
            message,
            authored_attempt: None,
        }
    }

    pub(in crate::daemon::run_store) fn for_attempt(
        message: &'a str,
        authored_attempt: u32,
    ) -> Self {
        Self {
            message,
            authored_attempt: Some(authored_attempt),
        }
    }
}

pub(in crate::daemon::run_store) struct TerminalDetails<'a> {
    pub(in crate::daemon::run_store) message: &'a str,
    pub(in crate::daemon::run_store) exit_code: Option<i32>,
}
