use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use monty::{LimitedTracker, MontyObject, MontyRun, PrintWriter, ResourceLimits};
use monty_type_checking::{SourceFile, type_check};
use tak_core::model::ModuleSpec;

use super::{
    DSL_STUBS, LoadOptions, PRELUDE,
    authored_source::{
        prepare_legacy_authored_source, runtime_input_names, runtime_inputs,
        validate_evaluated_version,
    },
    monty_deserializer::deserialize_from_monty,
};

pub(crate) fn eval_module_spec(path: &Path, options: &LoadOptions) -> Result<ModuleSpec> {
    let source = fs::read_to_string(path)?;
    let (prepared, admission) = prepare_legacy_authored_source(path, &source)?;
    let code = format!("{PRELUDE}\n\n{}", prepared.runtime_source);

    if options.enable_type_check {
        let script_name = path.to_string_lossy();
        let source = SourceFile::new(&prepared.authored_source, &script_name);
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

    let runner = MontyRun::new(code, &path.to_string_lossy(), runtime_input_names())
        .map_err(|e| anyhow!("failed to compile {}: {e}", path.display()))?;
    let value = runner
        .run(runtime_inputs(), tracker, PrintWriter::Disabled)
        .map_err(|e| anyhow!("failed to evaluate {}: {e}", path.display()))?;

    let evaluated_version = evaluated_module_version(&value)
        .map_err(|e| anyhow!("invalid module spec in {}: {e}", path.display()))?;
    validate_evaluated_version(path, &source, admission, evaluated_version)?;

    let module: ModuleSpec = deserialize_from_monty(&value)
        .map_err(|e| anyhow!("invalid module spec in {}: {e}", path.display()))?;

    Ok(module)
}

fn evaluated_module_version(value: &MontyObject) -> Result<u32> {
    let MontyObject::Dict(entries) = value else {
        bail!("unsupported Monty runtime value: {}", value.type_name());
    };
    for (key, version) in entries {
        if matches!(key, MontyObject::String(name) if name == "spec_version") {
            return deserialize_from_monty(version);
        }
    }
    Ok(1)
}
