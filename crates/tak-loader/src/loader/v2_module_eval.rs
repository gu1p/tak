use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use monty::{LimitedTracker, MontyRun, PrintWriter, ResourceLimits};
use monty_type_checking::{SourceFile, type_check};
use tak_core::v2::AuthoredModule;

use super::{
    DSL_STUBS_V2, LoadOptions, PRELUDE_V2,
    authored_source::{prepare_v2_authored_source, runtime_input_names, runtime_inputs},
    monty_deserializer::deserialize_from_monty,
    v2_wire, v2_wire_conversion,
};

pub(super) fn evaluate(path: &Path, options: &LoadOptions) -> Result<AuthoredModule> {
    let source = fs::read_to_string(path)?;
    let prepared = prepare_v2_authored_source(path, &source)?;
    type_check_authored(path, options, &prepared.authored_source)?;
    let code = format!("{PRELUDE_V2}\n\n{}", prepared.runtime_source);
    let limits = ResourceLimits::new()
        .max_duration(Duration::from_secs(2))
        .max_memory(64 * 1024 * 1024)
        .max_allocations(200_000);
    let runner = MontyRun::new(code, &path.to_string_lossy(), runtime_input_names())
        .map_err(|error| anyhow!("failed to compile {}: {error}", path.display()))?;
    let value = runner
        .run(
            runtime_inputs(),
            LimitedTracker::new(limits),
            PrintWriter::Disabled,
        )
        .map_err(|error| anyhow!("failed to evaluate {}: {error}", path.display()))?;
    let wire: v2_wire::Module = deserialize_from_monty(&value)
        .map_err(|error| anyhow!("invalid v2 module spec in {}: {error}", path.display()))?;
    v2_wire_conversion::into_domain(wire)
        .map_err(|error| anyhow!("invalid v2 module spec in {}: {error}", path.display()))
}

fn type_check_authored(path: &Path, options: &LoadOptions, authored_source: &str) -> Result<()> {
    if !options.enable_type_check {
        return Ok(());
    }
    let script_name = path.to_string_lossy();
    let source = SourceFile::new(authored_source, &script_name);
    let stubs = SourceFile::new(DSL_STUBS_V2, "tak_dsl_v2.pyi");
    match type_check(&source, Some(&stubs)) {
        Ok(None) => Ok(()),
        Ok(Some(diagnostics)) => bail!("type errors in {}:\n{diagnostics}", path.display()),
        Err(error) => bail!("type-checking failed for {}: {error}", path.display()),
    }
}
