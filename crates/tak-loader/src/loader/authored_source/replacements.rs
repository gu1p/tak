pub(super) fn execution_method_replacement(name: &str) -> Option<&'static str> {
    match name {
        "Local" => Some("Execution_Local"),
        "Remote" => Some("Execution_Remote"),
        "Policy" => Some("Execution_Policy"),
        "Session" => Some("Execution_Session"),
        _ => None,
    }
}

pub(super) fn runtime_method_replacement(name: &str) -> Option<&'static str> {
    match name {
        "Host" => Some("Runtime_Host"),
        "Image" => Some("Runtime_Image"),
        "Dockerfile" => Some("Runtime_Dockerfile"),
        _ => None,
    }
}

pub(super) fn transport_method_replacement(name: &str) -> Option<&'static str> {
    match name {
        "DirectHttps" => Some("Transport_DirectHttps"),
        "Any" => Some("Transport_Any"),
        "TorOnionService" => Some("Transport_TorOnionService"),
        _ => None,
    }
}

pub(super) fn session_reuse_method_replacement(name: &str) -> Option<&'static str> {
    match name {
        "Workspace" => Some("SessionReuse_Workspace"),
        "Paths" => Some("SessionReuse_Paths"),
        _ => None,
    }
}

pub(super) fn scope_constant_replacement(name: &str) -> Option<&'static str> {
    match name {
        "Machine" => Some("_Scope_Machine"),
        "User" => Some("_Scope_User"),
        "Project" => Some("_Scope_Project"),
        "Worktree" => Some("_Scope_Worktree"),
        _ => None,
    }
}

pub(super) fn hold_constant_replacement(name: &str) -> Option<&'static str> {
    match name {
        "During" => Some("_Hold_During"),
        "AtStart" => Some("_Hold_AtStart"),
        _ => None,
    }
}

pub(super) fn queue_discipline_constant_replacement(name: &str) -> Option<&'static str> {
    match name {
        "Fifo" => Some("_QueueDiscipline_Fifo"),
        "Priority" => Some("_QueueDiscipline_Priority"),
        _ => None,
    }
}

pub(super) fn session_lifetime_constant_replacement(name: &str) -> Option<&'static str> {
    match name {
        "PerRun" => Some("_SessionLifetime_PerRun"),
        _ => None,
    }
}
