CARGO ?= cargo
NPM ?= npm
NPX ?= npx
TARGET ?=
TARGET_FLAG := $(if $(TARGET),--target $(TARGET),)

BINARY_NAME := oxdoc
DOCS_PORT ?= 3000
COVERAGE_THRESHOLD ?= 95

.PHONY: help all ci ci-rust prepare-commit pre-push scripts-test
.PHONY: fmt fmt-check check clippy lint test doctest coverage coverage-html coverage-lcov audit
.PHONY: build build-release release build-musl musl docs docs-serve docs-check docs-links docs-schemas-check install-tools clean clean-coverage

help:
	@echo "oxdoc development targets"
	@echo ""
	@echo "  make fmt              Format Rust code"
	@echo "  make fmt-check        Check Rust formatting"
	@echo "  make check            cargo check across workspace"
	@echo "  make clippy           Run clippy with warnings denied"
	@echo "  make test             Run all tests"
	@echo "  make doctest          Run Rust doctests"
	@echo "  make audit            Check Cargo.lock for RustSec advisories"
	@echo "  make coverage         Enforce line coverage >= $(COVERAGE_THRESHOLD)%"
	@echo "  make coverage-html    Generate HTML coverage report"
	@echo "  make coverage-lcov    Generate LCOV coverage report"
	@echo "  make ci-rust          Run the Rust gates used by GitHub Actions"
	@echo "  make docs             Serve Docsify locally"
	@echo "  make docs-check       Validate Docsify serves locally"
	@echo "  make docs-links       Validate README and Docsify Markdown links"
	@echo "  make scripts-test     Test release/install helper scripts"
	@echo "  make build-release    Build optimized release binary"
	@echo "  make build-musl       Build static Linux musl binary"
	@echo "  make ci               Run the full local CI gate"

all: ci

ci: ci-rust scripts-test docs-check docs-links docs-schemas-check build-release
	@echo "All CI checks passed."

ci-rust: fmt-check check clippy test doctest coverage

prepare-commit: ci

pre-push: ci

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

check:
	$(CARGO) check --workspace --all-features --all-targets $(TARGET_FLAG)

clippy:
	$(CARGO) clippy --workspace --all-features --all-targets $(TARGET_FLAG) -- -D warnings

lint: clippy

test:
	$(CARGO) test --workspace --all-features --all-targets $(TARGET_FLAG)

doctest:
	$(CARGO) test --doc --workspace --all-features $(TARGET_FLAG)

scripts-test:
	sh -n install.sh tests/install.sh scripts/render-homebrew-formula.sh tests/homebrew_formula.sh
	sh tests/install.sh
	sh tests/homebrew_formula.sh

audit:
	$(CARGO) audit

coverage:
	$(CARGO) llvm-cov --workspace --all-features --all-targets --fail-under-lines $(COVERAGE_THRESHOLD) --summary-only

coverage-html:
	$(CARGO) llvm-cov --workspace --all-features --all-targets --html --output-dir target/coverage/html
	@echo "Coverage report: target/coverage/html/index.html"

coverage-lcov:
	@mkdir -p target/coverage
	$(CARGO) llvm-cov --workspace --all-features --all-targets --lcov --output-path target/coverage/lcov.info
	@echo "Coverage report: target/coverage/lcov.info"

build:
	$(CARGO) build --workspace --all-features $(TARGET_FLAG)

build-release:
	$(CARGO) build --workspace --all-features --release $(TARGET_FLAG)

release: build-release

build-musl:
	$(CARGO) build --workspace --all-features --release --target x86_64-unknown-linux-musl

musl: build-musl

docs: docs-serve

docs-serve:
	$(NPX) --yes docsify-cli@4 serve docs --port $(DOCS_PORT)

docs-check:
	@$(NPX) --yes docsify-cli@4 serve docs --port $(DOCS_PORT) >/tmp/oxdoc-docs.log 2>&1 & \
	server_pid=$$!; \
	trap 'kill $$server_pid 2>/dev/null || true' EXIT; \
	for attempt in $$(seq 1 30); do \
		if curl --fail --silent http://127.0.0.1:$(DOCS_PORT)/ >/tmp/oxdoc-docs.html; then break; fi; \
		sleep 1; \
	done; \
	cat /tmp/oxdoc-docs.log; \
	grep "oxdoc documentation" /tmp/oxdoc-docs.html

docs-links:
	@find README.md docs -name '*.md' -print0 | xargs -0 $(NPX) --yes markdown-link-check@3 --config .markdown-link-check.json

docs-schemas-check:
	@diff -ru schemas/v1 docs/schemas/v1

install-tools:
	$(CARGO) install cargo-llvm-cov --locked

clean-coverage:
	$(CARGO) llvm-cov clean --workspace
	rm -rf target/coverage

clean:
	$(CARGO) clean
