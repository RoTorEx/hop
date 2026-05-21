.PHONY: install deps-update build typecheck lint fmt test check run version release release-bump release-tag release-publish vibe-kernel-set vibe-kernel-path vibe-pull

install:
	cargo fetch --locked

deps-update:
	cargo update

build:
	cargo build --release --locked

typecheck:
	cargo check --locked --all-targets

lint:
	cargo clippy --locked --all-targets -- -D warnings

fmt:
	cargo fmt --all

test:
	cargo test --locked --all-targets

check:
	cargo fmt --all -- --check
	cargo check --locked --all-targets
	cargo clippy --locked --all-targets -- -D warnings
	cargo test --locked --all-targets
	cargo build --release --locked

run:
	cargo run --locked --

version:
	cargo run --locked -- --version

release:
	scripts/release.sh

release-bump:
	scripts/release-bump.sh

release-tag:
	@if [ -n "$$(git status --porcelain)" ]; then echo "ERROR: commit or stash changes before tagging."; exit 1; fi
	@version="$$(awk -F'"' '/^version = / { print $$2; exit }' Cargo.toml)"; \
	if [ -z "$$version" ]; then echo "ERROR: could not read version."; exit 1; fi; \
	if git rev-parse "v$$version" >/dev/null 2>&1; then echo "ERROR: tag v$$version already exists."; exit 1; fi; \
	git tag -a "v$$version" -m "v$$version"; \
	echo "Created tag v$$version"

release-publish:
	git push origin main --follow-tags

vibe-kernel-path:
	@if [ ! -f ".vibe/KERNEL_SOURCE" ]; then \
		echo "Missing .vibe/KERNEL_SOURCE."; \
		echo "Run: make vibe-kernel-set"; \
		exit 1; \
	fi; \
	printf "%s\n" "$$(cat .vibe/KERNEL_SOURCE)"

vibe-kernel-set:
	@mkdir -p .vibe; \
	if [ -n "$(KERNEL)" ]; then kernel_root="$(KERNEL)"; else \
		printf "Kernel path (absolute, contains tools/vibe-pull): " ; \
		read -r kernel_root; \
	fi; \
	if [ -z "$$kernel_root" ]; then echo "ERROR: empty path." >&2; exit 1; fi; \
	case "$$kernel_root" in /*) ;; *) echo "ERROR: must be an absolute path." >&2; exit 1;; esac; \
	if [ ! -f "$$kernel_root/tools/vibe-pull" ]; then echo "ERROR: cannot find $$kernel_root/tools/vibe-pull" >&2; exit 1; fi; \
	printf "%s\n" "$$kernel_root" > .vibe/KERNEL_SOURCE; \
	echo "Wrote .vibe/KERNEL_SOURCE"

vibe-pull:
	@if [ ! -f ".vibe/KERNEL_SOURCE" ]; then \
		echo "Missing .vibe/KERNEL_SOURCE."; \
		echo "Run: make vibe-kernel-set"; \
		exit 1; \
	fi; \
	kernel_root="$$(cat .vibe/KERNEL_SOURCE)"; \
	if [ ! -f "$$kernel_root/tools/vibe-pull" ]; then \
		echo "ERROR: cannot find $$kernel_root/tools/vibe-pull"; \
		exit 1; \
	fi; \
	python3 "$$kernel_root/tools/vibe-pull" .
