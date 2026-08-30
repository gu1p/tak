use crate::v2::PassEnv;

use super::ResolvedRunError;

pub(super) fn validate(names: &[String]) -> Result<(), ResolvedRunError> {
    let canonical =
        PassEnv::new(names).map_err(|error| ResolvedRunError::new(error.to_string()))?;
    if canonical.as_strs() != names.iter().map(String::as_str).collect::<Vec<_>>() {
        return Err(ResolvedRunError::new(
            "environment names must be sorted and unique",
        ));
    }
    Ok(())
}
