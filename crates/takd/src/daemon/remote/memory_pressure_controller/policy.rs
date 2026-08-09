use super::pressure::PressureState;

/// One takd container, distilled from the Docker list response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManagedContainer {
    pub(super) id: String,
    /// Docker `Created` unix timestamp; newest = largest.
    pub(super) created: i64,
    /// Carries a nonzero `tak.timeout_s` label — pausing it would let its
    /// wall-clock timeout fail the step, so it must never be paused.
    pub(super) has_timeout: bool,
    /// Engine state is `paused` (frozen) rather than `running`.
    pub(super) paused: bool,
}

/// Pick container ids to pause this tick from the currently-running set:
/// newest-first, never the oldest running container, never timeout-bearing, and
/// never enough to drop the running count below `min_running`. At most
/// `max_to_pause`.
///
/// ```no_run
/// # // Reason: operates on the private `ManagedContainer` type, not reachable from a doctest.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn select_pause_victims(
    running: &[ManagedContainer],
    min_running: usize,
    max_to_pause: usize,
) -> Vec<String> {
    if running.len() <= min_running {
        return Vec::new();
    }
    let budget = (running.len() - min_running).min(max_to_pause);
    if budget == 0 {
        return Vec::new();
    }
    // Protect the oldest running container so memory can always drain.
    let oldest_id = running
        .iter()
        .min_by(|a, b| a.created.cmp(&b.created).then(a.id.cmp(&b.id)))
        .map(|c| c.id.as_str());
    let mut candidates: Vec<&ManagedContainer> = running
        .iter()
        .filter(|c| !c.has_timeout)
        .filter(|c| Some(c.id.as_str()) != oldest_id)
        .collect();
    // Newest first; id as a stable tiebreaker.
    candidates.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    candidates
        .into_iter()
        .take(budget)
        .map(|c| c.id.clone())
        .collect()
}

/// Pick the paused container to resume: newest-created first (resume the most
/// recently started task first), mirroring the newest-first pause policy.
///
/// ```no_run
/// # // Reason: operates on the private `ManagedContainer` type, not reachable from a doctest.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn select_unpause_target(paused: &[ManagedContainer]) -> Option<String> {
    paused
        .iter()
        .max_by(|a, b| a.created.cmp(&b.created).then(a.id.cmp(&b.id)))
        .map(|c| c.id.clone())
}

/// What the controller should do this tick (admission-hold is handled separately,
/// before any fallible Docker work).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TickAction {
    Pause(Vec<String>),
    Unpause(String),
    None,
}

/// Pure tick decision over the current engine state.
///
/// **Forced progress** takes priority in every band: if fewer than `min_running`
/// containers are running while work is paused, unpause one — even under
/// Emergency. Without this, pausing down to the protected runner and then having
/// that runner finish (while memory is still below the resume watermark, since
/// paused RSS is retained) would freeze the node forever. Forced progress
/// guarantees at least `min_running` always running, so memory can always drain.
///
/// ```no_run
/// # // Reason: operates on the private `PressureState`/`ManagedContainer` types, not reachable from a doctest.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn decide(
    state: PressureState,
    running: &[ManagedContainer],
    paused: &[ManagedContainer],
    min_running: usize,
) -> TickAction {
    if running.len() < min_running
        && let Some(id) = select_unpause_target(paused)
    {
        return TickAction::Unpause(id);
    }
    match state {
        PressureState::Emergency => match select_pause_victims(running, min_running, usize::MAX) {
            victims if victims.is_empty() => TickAction::None,
            victims => TickAction::Pause(victims),
        },
        PressureState::Pause => {
            match select_pause_victims(running, min_running, 1)
                .into_iter()
                .next()
            {
                Some(id) => TickAction::Pause(vec![id]),
                None => TickAction::None,
            }
        }
        PressureState::Resume => match select_unpause_target(paused) {
            Some(id) => TickAction::Unpause(id),
            None => TickAction::None,
        },
        PressureState::Normal => TickAction::None,
    }
}
