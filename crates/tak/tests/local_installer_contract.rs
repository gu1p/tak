//! Contract tests for the local source installer script.

#[path = "local_installer_contract/default_install.rs"]
mod default_install;
#[path = "local_installer_contract/fixture.rs"]
mod fixture;
#[path = "local_installer_contract/fixture_run.rs"]
mod fixture_run;
#[path = "local_installer_contract/shell_path.rs"]
mod shell_path;
#[path = "local_installer_contract/target_directory.rs"]
mod target_directory;

use fixture::{InstallerFixture, PathMode};
