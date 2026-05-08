const ADJECTIVES: [&str; 32] = [
    "able", "brisk", "calm", "clear", "fair", "fast", "firm", "fresh", "kind", "level", "lucky",
    "plain", "prime", "quick", "ready", "right", "sharp", "solid", "steady", "still", "sure",
    "swift", "tidy", "true", "vivid", "warm", "wide", "wise", "bright", "direct", "gentle",
    "honest",
];

const NOUNS: [&str; 32] = [
    "anchor", "bridge", "brook", "field", "forge", "harbor", "hill", "lantern", "marker", "meadow",
    "node", "path", "ridge", "river", "signal", "stone", "summit", "track", "trail", "valley",
    "vista", "way", "work", "yard", "beam", "gate", "grid", "line", "port", "root", "span", "wire",
];

pub fn remote_alias_for_node_id(node_id: &str) -> String {
    let hash = stable_hash(node_id.trim().as_bytes());
    let left = ADJECTIVES[(hash as usize) % ADJECTIVES.len()];
    let right = NOUNS[((hash >> 16) as usize) % NOUNS.len()];
    format!("{left}-{right}")
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::remote_alias_for_node_id;

    #[test]
    fn alias_is_stable_and_human_readable() {
        assert_eq!(
            remote_alias_for_node_id("builder-node-123456"),
            remote_alias_for_node_id("builder-node-123456")
        );
        assert!(remote_alias_for_node_id("builder-node-123456").contains('-'));
    }
}
