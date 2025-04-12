## Configuration
## =============

.DEFAULT_GOAL := help
.DELETE_ON_ERROR: ;
.SECONDARY:
.SUFFIXES: ;


## Verbs
## =====

# TODO: Install mkhelp in the nix shell
help:
	@mkhelp $(firstword $(MAKEFILE_LIST))

## Checks
## ------

check: check_build check_docs check_format check_generated_files check_lint check_tests

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
	fd --type f '.*+.rs$$' \
	| xargs rustfmt --check \
		--config imports_granularity=Crate \
		--config group_imports=StdExternalCrate \
		--edition 2021
	cargo fmt --check
.PHONY: check_format_rs

## _
check_generated_files: Cargo.lock
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files

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

## _
check_tests:
	cargo test \
		--all-targets \
		--locked \
		--workspace
.PHONY: check_tests

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
	fd --type f '.*+.rs$$' \
	| xargs rustfmt \
		--config imports_granularity=Crate \
		--config group_imports=StdExternalCrate \
		--edition 2021
	cargo fmt
.PHONY: fix_format_rs

## _
fix_lint:
	cargo clippy --fix
.PHONY: fix_lint


## Nouns
## =====

Cargo.lock: $(wildcard crates/*/Cargo.toml)
	cargo metadata --format-version=1 > /dev/null
