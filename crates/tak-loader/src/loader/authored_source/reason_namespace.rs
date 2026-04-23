use monty::MontyObject;

const REASON_FIELDS: [&str; 5] = [
    "SIDE_EFFECTING_TASK",
    "NO_REMOTE_REACHABLE",
    "LOCAL_CPU_HIGH_ARM_IDLE",
    "LOCAL_CPU_HIGH",
    "DEFAULT_LOCAL_POLICY",
];

pub(super) fn reason_namespace() -> MontyObject {
    MontyObject::NamedTuple {
        type_name: "Reason".to_owned(),
        field_names: REASON_FIELDS
            .iter()
            .map(|field| (*field).to_owned())
            .collect(),
        values: REASON_FIELDS
            .iter()
            .map(|field| MontyObject::String((*field).to_owned()))
            .collect(),
    }
}
