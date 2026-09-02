use anyhow::Result;
use tak_core::v2::Step;
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn repo_root_file_checks_cover_all_files_without_git_metadata() -> Result<()> {
    let root = inspect_authored_root_module(super::repo_root(), &LoadOptions::default())?;

    for task_name in ["//:line-limits-check", "//:src-test-separation-check"] {
        let task = root
            .module
            .tasks
            .iter()
            .find(|task| task.name == task_name)
            .unwrap_or_else(|| panic!("missing {task_name} task"));
        let [Step::Cmd { env, .. }] = task.steps.as_slice() else {
            panic!("{task_name} should have one command step");
        };

        assert_eq!(
            env.get("TAK_LINE_MODE").map(String::as_str),
            Some("all"),
            "daemon v2 workspaces omit .git, so {task_name} must inspect all Rust files"
        );
    }
    Ok(())
}
