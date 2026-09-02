use anyhow::Result;
use tak_core::v2::TaskContext;

pub(super) fn paths(context: Option<&mut TaskContext>, package: &str) -> Result<()> {
    let Some(context) = context else {
        return Ok(());
    };
    for value in context
        .roots
        .iter_mut()
        .chain(&mut context.ignored_paths)
        .chain(&mut context.include)
    {
        *value = super::normalize::anchored(package, value)?;
    }
    Ok(())
}
