use std::fs;
use std::path::Path;

pub(super) fn write_root_and_app_tasks(root: &Path, app_tasks: &str) {
    let app_dir = root.join("apps/web");
    fs::create_dir_all(&app_dir).expect("mkdir");
    fs::write(
        root.join("TASKS.py"),
        "SPEC = module_spec(\n  includes=[path(\"apps/web\")],\n  tasks=[],\n)\nSPEC\n",
    )
    .expect("write root tasks");
    fs::write(app_dir.join("TASKS.py"), app_tasks).expect("write app tasks");
}
