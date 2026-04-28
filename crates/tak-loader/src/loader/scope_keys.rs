use std::env;
use std::path::Path;

use tak_core::model::{LimiterDef, LimiterKey, LimiterRef, Scope};

pub(crate) fn with_scope_key(reference: &LimiterRef, project_id: &str, root: &Path) -> LimiterRef {
    LimiterRef {
        name: reference.name.clone(),
        scope: reference.scope.clone(),
        scope_key: scope_key_for(&reference.scope, project_id, root),
    }
}

pub(crate) fn limiter_key_for_limiter(
    limiter: &LimiterDef,
    project_id: &str,
    root: &Path,
) -> LimiterKey {
    match limiter {
        LimiterDef::Resource { name, scope, .. }
        | LimiterDef::Lock { name, scope }
        | LimiterDef::RateLimit { name, scope, .. }
        | LimiterDef::ProcessCap { name, scope, .. } => LimiterKey {
            scope: scope.clone(),
            scope_key: scope_key_for(scope, project_id, root),
            name: name.clone(),
        },
    }
}

pub(crate) fn scope_label(scope: &Scope) -> &'static str {
    match scope {
        Scope::Machine => "machine",
        Scope::User => "user",
        Scope::Project => "project",
        Scope::Worktree => "worktree",
    }
}

pub(crate) fn scope_key_label(scope_key: &Option<String>) -> String {
    scope_key.as_deref().map_or_else(
        || "scope_key=(none)".to_string(),
        |value| format!("scope_key={value}"),
    )
}

pub(crate) fn scope_key_for(scope: &Scope, project_id: &str, root: &Path) -> Option<String> {
    match scope {
        Scope::Machine => None,
        Scope::User => env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .ok()
            .or(Some("unknown".to_string())),
        Scope::Project => Some(project_id.to_string()),
        Scope::Worktree => Some(root.to_string_lossy().into_owned()),
    }
}
