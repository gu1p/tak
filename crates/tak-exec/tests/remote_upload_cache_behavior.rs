//! End-to-end coverage for per-job workspace-upload caching: a cascading job uploads the same
//! repository to a node only once, later tasks reuse the cached blob, distinct content uploads
//! separately, a legacy node still works via inline submit, and a reaped blob is re-uploaded.
//!
//! These run against the recording fake node, which now serves the real upload protocol
//! (begin/append/finish) by default and records each `begin` so we can prove the upload count.

#![allow(clippy::await_holding_lock)]

mod dedup;
mod distinct_content;
mod edge_cases;
mod helpers;
