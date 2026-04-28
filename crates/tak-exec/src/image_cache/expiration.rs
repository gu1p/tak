use super::store::CacheEntry;

pub(super) fn entry_is_expired_mutable(
    entry: &CacheEntry,
    now_ms: i64,
    mutable_tag_ttl_secs: u64,
) -> bool {
    entry.source_kind == "mutable"
        && mutable_tag_is_expired_at(entry.last_refreshed_at_ms, now_ms, mutable_tag_ttl_secs)
}

pub(super) fn mutable_tag_is_expired_at(
    last_refreshed_at_ms: i64,
    now_ms: i64,
    ttl_secs: u64,
) -> bool {
    let ttl_ms = i64::try_from(ttl_secs.saturating_mul(1000)).unwrap_or(i64::MAX);
    now_ms.saturating_sub(last_refreshed_at_ms) >= ttl_ms
}

pub(super) fn eviction_age(entry: &CacheEntry, expired_mutable: bool) -> i64 {
    if expired_mutable {
        entry.last_refreshed_at_ms
    } else {
        entry.last_used_at_ms
    }
}
