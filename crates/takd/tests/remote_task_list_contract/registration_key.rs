pub(super) trait SubmitRegistrationKey {
    fn idempotency_key(&self) -> &str;
}

impl SubmitRegistrationKey for takd::SubmitRegistration {
    fn idempotency_key(&self) -> &str {
        match self {
            Self::Created { idempotency_key } | Self::Attached { idempotency_key } => {
                idempotency_key
            }
        }
    }
}
