#[derive(Default)]
pub(super) struct PeerResources {
    pub(super) cpu_total: Option<f64>,
    pub(super) memory_total_mb: Option<u64>,
}

impl PeerResources {
    pub(super) fn parse(summary: &str) -> Self {
        let mut resources = Self::default();
        for part in summary.split_whitespace() {
            if let Some(value) = part.strip_prefix("cpu_total=") {
                resources.cpu_total = value.parse::<f64>().ok();
            }
            if let Some(value) = part.strip_prefix("memory_total_mb=") {
                resources.memory_total_mb = value.parse::<u64>().ok();
            }
        }
        resources
    }
}
