use tak_core::model::{
    Hold, LimiterDef, LimiterKey, LimiterRef, NeedDef, Scope, TaskLabel, WorkspaceSpec,
};

pub fn add_ui_lock_need(spec: &mut WorkspaceSpec, label: &TaskLabel) {
    spec.limiters.insert(
        LimiterKey {
            scope: Scope::Machine,
            scope_key: None,
            name: "ui_lock".into(),
        },
        LimiterDef::Lock {
            name: "ui_lock".into(),
            scope: Scope::Machine,
        },
    );
    spec.tasks
        .get_mut(label)
        .expect("task")
        .needs
        .push(NeedDef {
            limiter: LimiterRef {
                name: "ui_lock".into(),
                scope: Scope::Machine,
                scope_key: None,
            },
            slots: 1.0,
            hold: Hold::During,
        });
}
