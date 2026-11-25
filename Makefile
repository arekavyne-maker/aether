SHELL := /bin/bash
.DEFAULT_GOAL := help

.PHONY: help build test python-test fitness dark-build baseline docs-check clean app app-smoke python-fetch wheels-fetch

help:
	@echo "Aether development targets:"
	@echo "  make build       - cargo build --workspace"
	@echo "  make test        - cargo test --workspace"
	@echo "  make python-test - pytest for SDK, geometry, requirements, plugins"
	@echo "  make fitness     - architecture fitness pipeline"
	@echo "  make dark-build  - full egress-free CI"
	@echo "  make baseline    - record test/benchmark evidence under docs/verification/baseline"
	@echo "  make docs-check  - relative Markdown link check"
	@echo "  make app         - build dist/Aether.app (macOS)"
	@echo "  make app-smoke   - headless bundle smoke test"
	@echo "  make python-fetch - fetch the relocatable Python runtime"
	@echo "  make wheels-fetch - fetch pinned runtime wheels"
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

app:
	bash scripts/package/macos-app.sh

app-smoke:
	./dist/Aether.app/Contents/MacOS/Aether --smoke-test --project .

python-fetch:
	bash scripts/package/fetch-python.sh

wheels-fetch:
	bash scripts/package/fetch-wheels.sh

baseline:
	@mkdir -p docs/verification/baseline/$$(date +%Y-%m-%d)
	@set -o pipefail; cargo test --workspace 2>&1 | tee docs/verification/baseline/$$(date +%Y-%m-%d)/cargo-test.txt | tail -20
	@set -o pipefail; python3 -m pytest -q 2>&1 | tee docs/verification/baseline/$$(date +%Y-%m-%d)/pytest.txt | tail -20
	@set -o pipefail; python3 infra/fitness/run.py 2>&1 | tee docs/verification/baseline/$$(date +%Y-%m-%d)/fitness.txt | tail -20
	@echo "Baseline recorded under docs/verification/baseline/$$(date +%Y-%m-%d)/"

clean:
	cargo clean
	rm -rf benchmarks infra/run/_mvr05
