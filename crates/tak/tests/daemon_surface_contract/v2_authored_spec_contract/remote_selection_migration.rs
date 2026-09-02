use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn v2_shuffle_points_to_balanced_and_balanced_reaches_validation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let env = BTreeMap::from([(
        "XDG_STATE_HOME".into(),
        temp.path().join("state").display().to_string(),
    )]);
    let shuffle = temp.path().join("shuffle");
    write_tasks(
        &shuffle,
        "POISON = 1 / 0\nSELECTION = RemoteSelection.Shuffle()\nSPEC = module_spec(spec_version=2, tasks=[])\nSPEC\n",
    )
    .expect("write shuffle tasks");

    let output = run_tak_output(&shuffle, &["run", "//:check"], &env).expect("run shuffle");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr:\n{stderr}");
    assert!(stderr.contains("TASKS.py:2:"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("RemoteSelection.Shuffle() was removed in spec_version=2; use Balanced."),
        "stderr:\n{stderr}"
    );
    assert!(!stderr.contains("division"), "poison evaluated: {stderr}");

    let balanced = temp.path().join("balanced");
    write_tasks(
        &balanced,
        "SELECTION = RemoteSelection.Balanced()\nSPEC = module_spec(spec_version=2, tasks=[task('check', steps=[cmd('true')])], defaults=Defaults(execution=Execution.Remote(selection=SELECTION)))\nSPEC\n",
    )
    .expect("write balanced tasks");
    let output = run_tak_output(&balanced, &["run", "//:check"], &env).expect("run balanced");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr:\n{stderr}");
    assert!(
        stderr.contains("Local takd is unavailable"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("no client execution fallback"),
        "stderr:\n{stderr}"
    );
    assert!(!stderr.contains("Shuffle"), "stderr:\n{stderr}");
}
