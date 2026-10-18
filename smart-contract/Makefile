# Every target carries a `##` block saying WHY, not what. `make help` prints the first line of each.

.PHONY: help all build test fmt lint gate coverage wasm-size clean deploy

NETWORK ?= testnet
TARGET  := wasm32v1-none
WASM    := target/$(TARGET)/release/payment_request_registry.wasm

## The wasm ships to a network where every byte is paid for. This is the ceiling CI enforces.
MAX_WASM_BYTES ?= 262144

help:  ## Print every target with its purpose.
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  %-14s %s\n", $$1, $$2}'

all: fmt lint gate test build  ## Everything CI runs, in the order CI runs it.

build: ## Release wasm for wasm32v1-none, then report its size.
	cargo build --workspace --target $(TARGET) --release
	@ls -l $(WASM) | awk '{print "wasm: " $$5 " bytes"}'

wasm-size: build ## Fail if the wasm has grown past the ceiling.
	@size=$$(wc -c < $(WASM)); \
	if [ $$size -gt $(MAX_WASM_BYTES) ]; then \
		echo "wasm is $$size bytes, ceiling is $(MAX_WASM_BYTES)"; exit 1; \
	else echo "wasm $$size bytes, within $(MAX_WASM_BYTES)"; fi

test: ## The 16 mandatory cases plus the system tests. Exact error codes, never is_err().
	cargo test --workspace

fmt: ## Format. CI checks rather than writes.
	cargo fmt --all

lint: ## Clippy at CI strictness. A warning is a failure.
	cargo clippy --workspace --all-targets -- -D warnings

## The one property that lets this contract skip a security audit. Checked, never remembered.
## The self-test runs first, because a gate nobody has watched reject something is not a gate.
gate: ## Prove the registry contains no token call.
	@./scripts/no_custody_gate_selftest.sh
	@./scripts/no_custody_gate.sh

coverage: ## Line coverage for the registry crate. Committed as Deliverable 1 evidence.
	@mkdir -p .deploy
	cargo llvm-cov --workspace \
		--ignore-filename-regex '(src/test/)' \
		--summary-only | tee .deploy/coverage.txt

deploy: ## Deploy and initialise in ONE transaction. KEY=<stellar key name> required.
	KEY=$(KEY) NETWORK=$(NETWORK) ./scripts/deploy.sh

clean: ## Remove build output and deployment artefacts.
	cargo clean
	rm -rf .deploy/env
