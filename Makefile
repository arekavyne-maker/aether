SHELL := /bin/bash
.DEFAULT_GOAL := help

.PHONY: help build test python-test fitness dark-build baseline docs-check clean

help:
	@echo "Aether development targets:"
	@echo "  make build       - cargo build --workspace"
	@echo "  make test        - cargo test --workspace"
	@echo "  make python-test - pytest for SDK, geometry, requirements, plugins"
	@echo "  make fitness     - architecture fitness pipeline"
	@echo "  make dark-build  - full egress-free CI"
	@echo "  make baseline    - record test/benchmark evidence under docs/verification/baseline"
	@echo "  make docs-check  - relative Markdown link check"
	@echo "  make clean       - remove build and scratch artifacts"

build:
	cargo build --workspace

test:
	cargo test --workspace

python-test:
	python3 -m pytest

fitness:
	python3 infra/fitness/run.py

dark-build:
	bash infra/ci/dark-build.sh

docs-check:
	python3 infra/fitness/docs_links.py

baseline:
	@mkdir -p docs/verification/baseline/$$(date +%Y-%m-%d)
	@set -o pipefail; cargo test --workspace 2>&1 | tee docs/verification/baseline/$$(date +%Y-%m-%d)/cargo-test.txt | tail -20
	@set -o pipefail; python3 -m pytest -q 2>&1 | tee docs/verification/baseline/$$(date +%Y-%m-%d)/pytest.txt | tail -20
	@set -o pipefail; python3 infra/fitness/run.py 2>&1 | tee docs/verification/baseline/$$(date +%Y-%m-%d)/fitness.txt | tail -20
	@echo "Baseline recorded under docs/verification/baseline/$$(date +%Y-%m-%d)/"

clean:
	cargo clean
	rm -rf benchmarks infra/run/_mvr05
