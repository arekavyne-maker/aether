SHELL := /bin/bash
.DEFAULT_GOAL := help

.PHONY: help build test python-test fitness dark-build baseline docs-check clean app app-smoke app-windows app-linux app-all python-fetch wheels-fetch sign-check

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
	@echo "  make app-windows - portable ZIP + WiX MSI (PowerShell)"
	@echo "  make app-linux   - tar.gz + .deb + optional AppImage"
	@echo "  make app-all     - every available desktop bundle"
	@echo "  make app-smoke   - headless bundle smoke test"
	@echo "  make python-fetch - fetch the relocatable Python runtime"
	@echo "  make wheels-fetch - fetch pinned runtime wheels"
	@echo "  make sign-check  - assert signing credentials are configured"
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
	bash experimental/macos/macos-app.sh

app-smoke:
	./dist/Aether.app/Contents/MacOS/Aether --smoke-test --project .

app-windows:
	pwsh -NoProfile -File experimental/windows/windows-app.ps1

app-linux:
	bash experimental/linux/linux-app.sh

app-all: app app-windows app-linux

sign-check:
	@case "$$(uname -s)" in \
	  Darwin) test -n "$$AETHER_SIGNING_IDENTITY" || { echo "set AETHER_SIGNING_IDENTITY"; exit 1; } ;; \
	  Linux) test -n "$$AETHER_GPG_KEY" || { echo "set AETHER_GPG_KEY"; exit 1; } ;; \
	  *) test -n "$$AETHER_SIGNING_CERT" || { echo "set AETHER_SIGNING_CERT"; exit 1; } ;; \
	esac

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
