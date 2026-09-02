pub fn remote_alias_for_node_id(node_id: &str) -> String {
    tak_core::remote_alias_for_node_id(node_id)
}

#[cfg(test)]
mod remote_alias_tests;
