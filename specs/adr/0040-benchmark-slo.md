# ADR-0040: Benchmark and SLO methodology

**Status:** Accepted
**Date:** 2026-11-24

## Context

§17.2 requires benchmark suites with reference solution, assumptions,
sensitivity, tolerances, expected outputs and permitted versions. §21 requires
twelve performance categories and states that speed must be reported
separately from accuracy. Current benchmarks number eleven and do not cover
CLI latency, artifact throughput, plugin startup, restore duration, telemetry
ingest or agent latency.

## Decision

Establish a benchmark registry with two axes: scientific validity and
operational performance. Each benchmark declares its reference, tolerance,
version constraints and known limitations. SLOs are defined per deployment
profile after measurement, not invented. `make baseline` records results as
evidence. Fitness fails if a declared benchmark has no runnable command.

## Consequences

- A fast but inaccurate result cannot be presented as an optimisation.
- Regressions in latency or accuracy are visible in CI.
- SLO claims are backed by recorded measurements.
