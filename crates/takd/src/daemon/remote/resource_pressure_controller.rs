#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ResourcePressureState {
    #[default]
    Healthy,
    Pressure,
    Recovering,
}

impl ResourcePressureState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Pressure => "pressure",
            Self::Recovering => "recovering",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResourcePressureSnapshot {
    state: ResourcePressureState,
    episode_started_at_ms: Option<i64>,
    healthy_samples: usize,
}

impl ResourcePressureSnapshot {
    pub(crate) fn healthy() -> Self {
        Self::default()
    }

    pub(crate) fn pressure(episode_started_at_ms: i64) -> Self {
        Self {
            state: ResourcePressureState::Pressure,
            episode_started_at_ms: Some(episode_started_at_ms),
            healthy_samples: 0,
        }
    }

    pub(crate) fn recovering(episode_started_at_ms: i64, healthy_samples: usize) -> Self {
        Self {
            state: ResourcePressureState::Recovering,
            episode_started_at_ms: Some(episode_started_at_ms),
            healthy_samples,
        }
    }

    pub(crate) fn state_name(&self) -> &'static str {
        self.state.as_str()
    }

    pub(crate) fn episode_started_at_ms(&self) -> Option<i64> {
        self.episode_started_at_ms
    }

    pub(crate) fn healthy_samples(&self) -> usize {
        self.healthy_samples
    }
}
