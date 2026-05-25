.PHONY: audit check clean clippy coverage doc fmt fmt-fix integration machete patch-coverage test

CLIPPY_FLAGS := -D clippy::all
RUSTDOCFLAGS := -D warnings
PATCH_COVERAGE_BASE ?= main
PATCH_COVERAGE_FAIL_UNDER ?= 100
DIFF_COVER ?= diff-cover

check: fmt clippy test doc

fmt:
	cargo fmt --all --check

fmt-fix:
	cargo +nightly fmt --all

clippy:
	cargo clippy --all-targets --all-features -- $(CLIPPY_FLAGS)
	cargo clippy --lib --no-default-features -- $(CLIPPY_FLAGS)

test:
	cargo test --all-features
	cargo test --lib --no-default-features

integration:
	cargo test --all-features -- --ignored

doc:
	RUSTDOCFLAGS="$(RUSTDOCFLAGS)" cargo doc --all-features --no-deps
	RUSTDOCFLAGS="$(RUSTDOCFLAGS)" cargo doc --no-default-features --no-deps

coverage:
	cargo llvm-cov --workspace --all-features --fail-under-lines 90

patch-coverage:
	cargo llvm-cov --workspace --all-features --fail-under-lines 90 --lcov --output-path lcov.info
	$(DIFF_COVER) lcov.info --compare-branch=$(PATCH_COVERAGE_BASE) --fail-under=$(PATCH_COVERAGE_FAIL_UNDER)

audit:
	cargo audit

machete:
	cargo machete

clean:
	cargo clean
	rm -f lcov.info
