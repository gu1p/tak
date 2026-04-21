use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use monty::{LimitedTracker, MontyObject, MontyRun, PrintWriter, ResourceLimits};
use monty_type_checking::{SourceFile, type_check};
use serde_json::{Map, Value};
use tak_core::model::ModuleSpec;

use super::{DSL_STUBS, LoadOptions, PRELUDE};

pub(crate) fn eval_module_spec(path: &Path, options: &LoadOptions) -> Result<ModuleSpec> {
    let source = fs::read_to_string(path)?;
    let source = sanitize_canonical_v1_imports(&source);
    let code = format!("{PRELUDE}\n\n{source}");

    if options.enable_type_check {
        let script_name = path.to_string_lossy();
        let source = SourceFile::new(&code, &script_name);
        let stubs = SourceFile::new(DSL_STUBS, "tak_dsl.pyi");
        match type_check(&source, Some(&stubs)) {
            Ok(None) => {}
            Ok(Some(diagnostics)) => {
                bail!("type errors in {}:\n{}", path.display(), diagnostics);
            }
            Err(err) => {
                bail!("type-checking failed for {}: {err}", path.display());
            }
        }
    }

    let limits = ResourceLimits::new()
        .max_duration(Duration::from_secs(2))
        .max_memory(64 * 1024 * 1024)
        .max_allocations(200_000);
    let tracker = LimitedTracker::new(limits);

    let runner = MontyRun::new(code, &path.to_string_lossy(), Vec::new(), Vec::new())
        .map_err(|e| anyhow!("failed to compile {}: {e}", path.display()))?;
    let value = runner
        .run(Vec::new(), tracker, &mut PrintWriter::Disabled)
        .map_err(|e| anyhow!("failed to evaluate {}: {e}", path.display()))?;

    let json = monty_to_json(value)?;
    let module: ModuleSpec = serde_json::from_value(json)
        .map_err(|e| anyhow!("invalid module spec in {}: {e}", path.display()))?;

    if module.spec_version != 1 {
        bail!(
            "unsupported spec_version {} in {}",
            module.spec_version,
            path.display()
        );
    }

    Ok(module)
}

pub(crate) fn sanitize_canonical_v1_imports(source: &str) -> String {
    let mut output = Vec::new();
    let mut skipping_multiline_import = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if skipping_multiline_import {
            if trimmed.contains(')') {
                skipping_multiline_import = false;
            }
            continue;
        }

        let is_tak_import =
            trimmed.starts_with("from tak import") || trimmed.starts_with("from tak.remote import");
        if is_tak_import {
            if trimmed.contains('(') && !trimmed.contains(')') {
                skipping_multiline_import = true;
            }
            continue;
        }

        output.push(line);
    }

    let mut normalized = output.join("\n");
    normalized = normalized
        .replace("RemoteTransportMode.AnyTransport(", "AnyTransport(")
        .replace("RemoteTransportMode.DirectHttps(", "DirectHttps(")
        .replace("RemoteTransportMode.TorOnionService(", "TorOnionService(")
        .replace("ServiceAuth.from_env(", "ServiceAuth_from_env(")
        .replace(
            "WorkspaceTransferMode.REPO_ZIP_SNAPSHOT",
            "\"REPO_ZIP_SNAPSHOT\"",
        )
        .replace("ResultSyncMode.OUTPUTS_AND_LOGS", "\"OUTPUTS_AND_LOGS\"")
        .replace("Decision.remote_any(", "Decision_remote_any(")
        .replace("Decision.remote(", "Decision_remote(")
        .replace("Decision.local(", "Decision_local(")
        .replace("Reason.SIDE_EFFECTING_TASK", "REASON_SIDE_EFFECTING_TASK")
        .replace("Reason.NO_REMOTE_REACHABLE", "REASON_NO_REMOTE_REACHABLE")
        .replace(
            "Reason.LOCAL_CPU_HIGH_ARM_IDLE",
            "REASON_LOCAL_CPU_HIGH_ARM_IDLE",
        )
        .replace("Reason.LOCAL_CPU_HIGH", "REASON_LOCAL_CPU_HIGH")
        .replace("Reason.DEFAULT_LOCAL_POLICY", "REASON_DEFAULT_LOCAL_POLICY");
    if source.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

/// Converts a Monty runtime object into strict JSON-compatible `serde_json::Value`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn monty_to_json(value: MontyObject) -> Result<Value> {
    let json = match value {
        MontyObject::None => Value::Null,
        MontyObject::Bool(v) => Value::Bool(v),
        MontyObject::Int(v) => Value::Number(v.into()),
        MontyObject::BigInt(v) => {
            let as_i64 = v
                .to_string()
                .parse::<i64>()
                .map_err(|_| anyhow!("bigint value out of i64 range"))?;
            Value::Number(as_i64.into())
        }
        MontyObject::Float(v) => {
            let number = serde_json::Number::from_f64(v)
                .ok_or_else(|| anyhow!("non-finite float value is not JSON-compatible"))?;
            Value::Number(number)
        }
        MontyObject::String(v) => Value::String(v),
        MontyObject::List(items) => Value::Array(
            items
                .into_iter()
                .map(monty_to_json)
                .collect::<Result<Vec<_>>>()?,
        ),
        MontyObject::Dict(pairs) => {
            let mut map = Map::new();
            for (key, value) in pairs {
                let key_string = match key {
                    MontyObject::String(s) => s,
                    other => return Err(anyhow!("dict key must be a string, got {other:?}")),
                };
                map.insert(key_string, monty_to_json(value)?);
            }
            Value::Object(map)
        }
        other => return Err(anyhow!("non-JSON-compatible Monty value: {other:?}")),
    };

    Ok(json)
}
