# Convenience wrapper around cargo and the fluff CLI.
# Run `make` (or `make help`) for the list. C=... sets the byte to repeat,
# SIZE=... the decompressed size (k/m/g/t suffixes), FORMAT=both|gz|br which
# files to write, and NAME=... their name in $(OUT)/.

.DEFAULT_GOAL := help
.PHONY: help check build debug test run files verify install fmt clippy lint clean

C ?= a
SIZE ?= 1m
FORMAT ?= both
NAME ?= repeat
OUT ?= out

# cargo run rather than target/release/fluff, since ~/.cargo/config.toml may
# put the build somewhere else.
FLUFF := cargo run --release --quiet --

help:
	@echo "fluff — common tasks"
	@echo
	@echo "  make check      cargo check"
	@echo "  make build      cargo build --release"
	@echo "  make test       cargo test --release"
	@echo "  make run        cargo run --release -- --help"
	@echo "  make files      write $(OUT)/$(NAME).gz and .br (C=a SIZE=1m FORMAT=both|gz|br)"
	@echo "  make verify     decompress them and count the bytes"
	@echo "  make install    cargo install --path . — puts fluff on your PATH"
	@echo "  make lint       cargo fmt --check, then clippy with warnings as errors"

check:
	cargo check --all-targets
build:
	cargo build --release
debug:
	cargo build
test:
	cargo test --release
run:
	$(FLUFF) --help
files:
	@mkdir -p $(OUT)
	$(FLUFF) --char $(C) --size $(SIZE) --format $(FORMAT) --out $(OUT)/$(NAME)
verify:
	@test -n "$(wildcard $(OUT)/$(NAME).gz $(OUT)/$(NAME).br)" || { echo "No $(OUT)/$(NAME).gz or .br: run make files first"; exit 1; }
	@ls -l $(wildcard $(OUT)/$(NAME).gz $(OUT)/$(NAME).br)
	@if [ -f $(OUT)/$(NAME).gz ]; then printf "%s.gz expands to " $(NAME); gunzip -c $(OUT)/$(NAME).gz | wc -c; fi
	@if [ -f $(OUT)/$(NAME).br ]; then \
	  if command -v brotli >/dev/null; then printf "%s.br expands to " $(NAME); brotli -d -c $(OUT)/$(NAME).br | wc -c; \
	  else echo "(install the brotli CLI to check the .br file)"; fi; fi
install:
	cargo install --path .
fmt:
	cargo fmt
clippy:
	cargo clippy --all-targets -- -D warnings
lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
clean:
	cargo clean
	rm -rf $(OUT)
