//! Walking skeleton (b) — SQLite event-sourced graph at 10M-node scale.
//!
//! Validates the substrate that the full Engineering Graph engine will use
//! in Phase 1 (G1). The scale test inserts N node_created events, each
//! appended to the events log and projected to the nodes table, then
//! replays the current state.
//!
//! **Phase 0 measurement (10M nodes on x86_64, single thread):**
//!
//! | Metric | Result | Notes |
//! |---|---|---|
//! | Insert time | 565s (9.4 min) | batches of 10k; can be improved with prepared statements |
//! | Replay time | 6.4s | well within SDP §18 budget (<5 min) |
//! | Throughput | 17.7k events/s | baseline for Phase 1 tuning |
//! | DB size | 7.3 GB | ~750 bytes/node including events and nodes |
//!
//! **Phase 2 measurement (10M nodes on aarch64-apple-darwin, single thread,
//! 2026-09-04, infra/release/scale_10m_2026-09-04.json):**
//!
//! | Metric | Result | Notes |
//! |---|---|---|
//! | Insert time | 546.35 s (9.1 min) | same code path; aarch64 single thread is ~3% faster than x86_64 |
//! | Replay time | 6.30 s | within noise of the x86_64 number |
//! | Throughput | 18.3k events/s | marginally higher than x86_64 (different cache behaviour) |
//! | DB size | 7.26 GB | matches x86_64 (CAS is content-addressed) |
//!
//! The replay is the key metric (SDP §18 acceptance): it represents the
//! cost of materializing current state from the event log, which is the
//! hot path for graph queries. The aarch64 replay time of 6.30 s is
//! 47.6× faster than the SDP §18 Phase 0 budget of 300 s. The insert
//! time of 546.35 s is the SDP Phase 1 implied budget (5 min,
//! `*M events/s throughput stated in the README) — insert DOES NOT
//! meet that budget on this machine (1.82× over); see
//! `workflows/paper-run/gaps.md` AP-26.
//!
//! The replay is the key metric (SDP §18 acceptance): it represents the
//! cost of materializing current state from the event log, which is the
//! hot path for graph queries.
//!
//! Run:
//!   cargo build -p aether_graph
//!   ./target/debug/aether_graph_scale --nodes 10000000 --db /tmp/aether.db
//!
//! The 100k smoke test is in `scale::tests::smoke_in_memory`.

#[allow(dead_code)]
const _WALKING_SKELETON_B_DOC: &str = include_str!("../README.md");
