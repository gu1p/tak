/// Whether the recording node serves the real workspace-upload protocol (faithful to `takd`,
/// the default) or behaves like a legacy node that 404s `begin`, forcing the client to inline
/// the workspace into the submit request.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum UploadMode {
    Protocol,
    LegacyInline404,
}

#[derive(Clone, Copy)]
pub(super) struct UploadConfig {
    pub(super) mode: UploadMode,
    /// When true, an upload is reaped immediately after a submit references it, so the NEXT
    /// submit reusing the same blob receives 409 — exercising the client's re-upload fallback
    /// for blobs the cleanup janitor removes mid-job.
    pub(super) reap_after_reference: bool,
}

impl UploadConfig {
    pub(super) fn protocol() -> Self {
        Self {
            mode: UploadMode::Protocol,
            reap_after_reference: false,
        }
    }

    pub(super) fn legacy_inline() -> Self {
        Self {
            mode: UploadMode::LegacyInline404,
            reap_after_reference: false,
        }
    }

    pub(super) fn reaping() -> Self {
        Self {
            mode: UploadMode::Protocol,
            reap_after_reference: true,
        }
    }
}
