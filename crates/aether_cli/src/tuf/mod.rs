// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Phase 0 walking skeleton (c) — offline TUF bundle verification.
//!
//! Implements a minimal subset of TUF (The Update Framework) metadata,
//! adapted for offline media (ADR-0021):
//!
//! - `targets.json`     — list of artifacts with hashes and length
//! - `snapshot.json`    — frozen state of targets at a sequence number
//! - `timestamp.json`   — validity window + current snapshot sequence
//! - `root.json`        — signs the metadata keys
//!
//! All metadata is JSON, Ed25519-signed, and content-addressed. The verifier
//! refuses a bundle whose chain does not verify against the supplied root
//! (I-12, fail-visible).

pub mod verify;
