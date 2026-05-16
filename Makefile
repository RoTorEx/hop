.PHONY: install deps-update build watch typecheck lint fmt test check run release-bump release-tag release-publish vibe-kernel-set vibe-kernel-path vibe-pull

install:
	@echo "TODO: implement a real install command for this project."; exit 1

deps-update:
	@echo "TODO: implement a real dependency update flow for this project."; exit 1

build:
	@echo "TODO: implement a real build command for this project."; exit 1

watch:
	@echo "TODO: implement a real watch/dev command for this project."; exit 1

typecheck:
	@echo "TODO: implement a real typecheck command for this project."; exit 1

lint:
	@echo "TODO: implement lint only if real lint exists."; exit 1

fmt:
	@echo "TODO: implement formatting only if real formatter exists."; exit 1

test:
	@echo "TODO: implement tests only if real tests exist."; exit 1

check:
	@echo "TODO: implement 'make check' as a real verification command."; exit 1

run:
	@echo "TODO: implement a real run command for this project."; exit 1

release-bump:
	@echo "TODO: implement release bump only if this project actually releases."; exit 1

release-tag:
	@echo "TODO: implement release tagging only if this project actually releases."; exit 1

release-publish:
	@echo "TODO: implement release publish/deploy only if this project actually releases."; exit 1

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
