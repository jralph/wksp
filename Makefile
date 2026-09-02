BINARY_NAME := wksp
RELEASE_BIN := target/release/$(BINARY_NAME)
INSTALL_DIR := $(HOME)/.local/bin
INSTALL_PATH := $(INSTALL_DIR)/$(BINARY_NAME)

.PHONY: help build install link uninstall clean test fmt lint

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "%-10s %s\n", $$1, $$2}'

build: ## Build the release binary
	cargo build --release

install: build ## Build and symlink the binary into ~/.local/bin (must be on PATH)
	mkdir -p $(INSTALL_DIR)
	ln -sf $(abspath $(RELEASE_BIN)) $(INSTALL_PATH)
	@echo "Installed: $(INSTALL_PATH) -> $(abspath $(RELEASE_BIN))"
	@case ":$$PATH:" in \
		*":$(INSTALL_DIR):"*) ;; \
		*) echo "warning: $(INSTALL_DIR) is not on your PATH. Add it, e.g.: export PATH=\"$(INSTALL_DIR):\$$PATH\"" ;; \
	esac

link: install ## Alias for install

uninstall: ## Remove the symlinked binary from ~/.local/bin
	rm -f $(INSTALL_PATH)
	@echo "Removed $(INSTALL_PATH)"

clean: ## Remove build artifacts
	cargo clean

test: ## Run the test suite
	cargo test

fmt: ## Check formatting
	cargo fmt --check

lint: ## Run clippy with warnings as errors
	cargo clippy --all-targets -- -D warnings
