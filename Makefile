CARGO ?= cargo
TARGET ?=
TARGET_FLAG := $(if $(TARGET),--target $(TARGET),)

.PHONY: all fmt lint test build release musl clean

all: fmt lint test build

fmt:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test:
	$(CARGO) test --workspace

build:
	$(CARGO) build --workspace $(TARGET_FLAG)

release:
	$(CARGO) build --workspace --release $(TARGET_FLAG)

musl:
	$(CARGO) build --workspace --release --target x86_64-unknown-linux-musl

clean:
	$(CARGO) clean

