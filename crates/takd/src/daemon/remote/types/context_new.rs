use super::*;

impl RemoteNodeContext {
    pub fn new(node: NodeInfo, bearer_token: String, runtime_config: RemoteRuntimeConfig) -> Self {
        let tak_container_usage = SharedTakContainerUsage::default();
        let oversubscribe_x = runtime_config.admission_oversubscribe_x();
        let resource_policy = RemoteResourcePolicy::detected(&runtime_config);
        let envelope = resource_policy.envelope();
        let elastic_limits = resource_policy.resolve(None);
        let elastic_startup = resource_policy.startup_claim(&elastic_limits);
        let initial_host_usage =
            runtime_config
                .ignore_host_usage_for_tests()
                .then_some(HostUsageSample {
                    non_tak_usage: ResourceCapacity {
                        cpu_cores: 0.0,
                        memory_mb: 0,
                    },
                    available_memory_mb: u64::MAX,
                });
        let resource_admission = SharedResourceAdmission::new_with_resource_envelope(
            tak_container_usage.clone(),
            envelope,
            oversubscribe_x,
            elastic_startup,
            initial_host_usage,
        );
        Self {
            node: Arc::new(Mutex::new(node)),
            bearer_token,
            status_state: new_shared_node_status_state(tak_container_usage.clone()),
            active_executions: SharedActiveExecutions::default(),
            resource_admission,
            resource_policy,
            tak_container_usage,
            resource_pressure: Arc::new(Mutex::new(ResourcePressureSnapshot::healthy())),
            runtime_state: Arc::new(RemoteRuntimeState::new(runtime_config)),
            image_cache: None,
            state_root: None,
        }
    }

    pub fn with_image_cache_config(mut self, config: RemoteImageCacheRuntimeConfig) -> Self {
        self.image_cache = Some(config);
        self
    }
}
