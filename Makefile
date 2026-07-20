## Configuration
## =============

.DEFAULT_GOAL := help
.DELETE_ON_ERROR: ;
.SECONDARY:
.SUFFIXES: ;
.PHONY: .FORCE

## Verbs
## =====

help:
	@mkhelp $(firstword $(MAKEFILE_LIST))

## Compare the output from building example apps
replay_equivalence_examples:
	cargo run --locked -p rs4a-acap-build-tester -- \
		replay \
		crates/acap-build/tests/data
.PHONY: replay_equivalence_examples

## Compare the output from building generated apps
fuzz_equivalence:
	cargo run --locked -p rs4a-acap-build-tester -- \
		fuzz
.PHONY: fuzz_equivalence

## Checks
## ------

## _
check: check_t3 check_format_nix check_generated_files check_tests
.PHONY: check

check_t3: check_build check_docs check_format_rs check_generated_files_t3 check_lint check_tests_t3
.PHONY: check_t3

## _
check_build:
	cargo build \
		--locked \
		--workspace
.PHONY: check_build

## _
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc \
		--document-private-items \
		--locked \
		--no-deps \
		--workspace
.PHONY: check_docs

## _
check_format: check_format_nix check_format_rs
.PHONY: check_format

check_format_nix:
	fd --type f '.*+.nix$$' \
	| xargs nixfmt --check
.PHONY: check_format_nix

check_format_rs:
	cargo fmt \
		--check \
		-- \
		--config imports_granularity=Crate,group_imports=StdExternalCrate
.PHONY: check_format_rs

# TODO: Ensure unstaged files cause this to fail

## _
check_generated_files: \
	check_generated_files_t3 \
	snapshots/acap-build-disallowed-property \
	snapshots/acap-build-too-long-string
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files

check_generated_files_t3: \
	Cargo.lock \
	snapshots/acap-build-docs \
	snapshots/cli4a-docs \
	snapshots/device-finder-docs \
	snapshots/device-inventory-docs \
	snapshots/device-inventory-smoke-test \
	snapshots/device-manager-docs \
	snapshots/fimage-docs \
	snapshots/firmware-inventory-docs
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files_t3

## _
check_lint:
	cargo clippy \
		--all-targets \
		--locked \
		--no-deps \
		--workspace \
		-- \
		-Dwarnings
.PHONY: check_lint

# I have a vague memory of cassettes being less reproducible when order is preserved because the
# server returns JSON with inconsistent field ordering. So when I ran into ordering issues due
# to workspace feature unification I took a shortcut and split the test execution.
# TODO: Investigate if cassette tests can be made to run with `serde_json/preserve_order`

## _
check_tests: check_tests_t3
	cargo test \
		--all-targets \
		--locked \
		-p acap-build \
		-p rs4a-acap-build-tester \
		-p rs4a-eap \
		-- --ignored
.PHONY: check_tests

check_tests_t3:
	cargo test \
		--all-targets \
		--locked \
		--workspace \
		--exclude acap-build \
		--exclude rs4a-acap-build-tester \
		--exclude rs4a-eap
.PHONY: check_tests_t3

check_tests_mac_os: init-env.sh
	. ./init-env.sh && \
	export PATH="/opt/homebrew/opt/gnu-tar/libexec/gnubin:${PATH}" && \
	cargo test -p acap-build -- --ignored

## Fixes
## -----

## _
fix_format: fix_format_nix fix_format_rs ;
.PHONY: fix_format

fix_format_nix:
	fd --type f '.*+.nix$$' \
	| xargs nixfmt
.PHONY: fix_format_nix

fix_format_rs:
	cargo fmt \
	-- \
	--config imports_granularity=Crate,group_imports=StdExternalCrate
.PHONY: fix_format_rs

## _
fix_lint:
	cargo clippy --fix
.PHONY: fix_lint


## Nouns
## =====

Cargo.lock: $(wildcard crates/*/Cargo.toml)
	cargo metadata --format-version=1 > /dev/null

init-env.sh: bin/create-venv.sh
	$<

# The `acap-build` snapshot tests need the ACAP Native SDK. This Makefile assumes
# it is already installed and, if it is not at the default `/opt/axis`, that
# `ACAP_SDK_LOCATION` points at it. The Nix dev shell and `bin/create-venv.sh`
# both set this up; see the README for details.
snapshots/%: bin/%.sh .FORCE
	cargo build --bins
	PATH=$$(pwd)/target/debug:$$PATH \
	$< \
	> $@ 2>&1
