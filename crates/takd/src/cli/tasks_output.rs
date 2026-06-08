use std::path::Path;

use anyhow::Result;
use takd::agent::read_config;

mod client;
mod render;

pub(super) fn print_active_tasks(config_root: &Path, state_root: &Path) -> Result<()> {
    let config = read_config(config_root)?;
    let status = client::fetch_live_status(state_root, &config.bearer_token)?;
    print!("{}", render::render_active_tasks(&status));
    Ok(())
}
