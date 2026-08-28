use std::collections::HashMap;

pub(super) const OWNER_LABEL: &str = "tak.owner";
pub(super) const OWNER_VALUE: &str = "takd";
pub(super) const NODE_ID_LABEL: &str = "tak.node_id";

pub(super) fn labels_belong_to_node(
    labels: Option<&HashMap<String, String>>,
    node_id: &str,
) -> bool {
    let Some(labels) = labels else {
        return false;
    };
    labels.get(OWNER_LABEL).map(String::as_str) == Some(OWNER_VALUE)
        && labels.get(NODE_ID_LABEL).map(String::as_str) == Some(node_id)
}

pub(super) fn add_node_ownership_filter(filters: &mut HashMap<String, Vec<String>>, node_id: &str) {
    filters.insert(
        "label".to_string(),
        vec![
            format!("{OWNER_LABEL}={OWNER_VALUE}"),
            format!("{NODE_ID_LABEL}={node_id}"),
        ],
    );
}
