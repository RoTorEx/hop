# Rust Projects

Use this file only for Rust/Cargo project work.

Do not read it during non-Rust tasks.

## When to read

Read this file when the user asks to:

- bootstrap or standardize a Rust project;
- edit Rust Makefile targets;
- change Cargo build, check, lint, format, test, or run commands;
- change Cargo workspace layout or build artifact behavior.

## Build artifact rule

Rust projects must keep routine Cargo build artifacts outside the repository.

All Makefile targets that invoke Cargo must export this target directory:

```make
PROJECT_NAME := $(notdir $(CURDIR))
CONSTRUCTION_SIDE := $(HOME)/construction_side
CARGO_TARGET_DIR := $(CONSTRUCTION_SIDE)/$(PROJECT_NAME)/target
export CARGO_TARGET_DIR
```

The intended path is:

```text
~/construction_side/<project_name>/target
```

Use the repository directory basename as `<project_name>` unless the child project already has one stable local name documented in its Makefile.

## Command rules

- Expose routine Cargo work through plain Make targets: `make build`, `make test`, `make lint`, `make fmt`, `make check`, and `make run` when applicable.
- Do not ask humans or agents to remember `CARGO_TARGET_DIR=...` on the command line.
- Do not use project-local `target/` for routine commands.
- Keep `target/` in `.gitignore` anyway, because direct ad-hoc Cargo commands can still create it.
- Prefer Makefile-owned variables over shell-profile, direnv, or global Cargo config for this rule.
- Do not add `.cargo/config.toml` only to set `target-dir` unless the child project already uses Cargo config for other real project needs.
- Use one shared target directory per repository or Cargo workspace. Do not create per-crate target directories inside a workspace unless the project has separate independent workspaces.

## Makefile example

This is the expected shape for a Rust child project. Adapt native Cargo flags locally, but keep the target directory rule and plain public targets.

```make
.PHONY: cargo-target-dir build test lint fmt check run

PROJECT_NAME := $(notdir $(CURDIR))
CONSTRUCTION_SIDE := $(HOME)/construction_side
CARGO_TARGET_DIR := $(CONSTRUCTION_SIDE)/$(PROJECT_NAME)/target
export CARGO_TARGET_DIR

cargo-target-dir:
	@mkdir -p "$(CARGO_TARGET_DIR)"

build: cargo-target-dir
	cargo build

test: cargo-target-dir
	cargo test

lint: cargo-target-dir
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

check: fmt lint test

run: cargo-target-dir
	cargo run
```

## Standardization checklist

For an existing Rust child project:

1. Read `.vibe/kernel/COMMAND_INTERFACE.md` and this file.
2. Update `Makefile` so every Cargo-invoking target receives the exported `CARGO_TARGET_DIR`.
3. Keep public docs on `make ...` commands, not raw Cargo commands.
4. Keep `target/` ignored in `.gitignore`.
5. Run `make check` or the closest real verification command.
