export CARGO_TARGET_DIR ?= ./target
export CARGO_NET_GIT_FETCH_WITH_CLI ?= true

DESTDIR ?= /
PREFIX ?= /usr

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: build-debug
build-debug:
	cargo build

.PHONY: install
install:
	install -Dm755 target/release/elara-launcher $(DESTDIR)$(PREFIX)/bin/elara-launcher

.PHONY: install-debug
install-debug:
	install -Dm755 target/debug/elara-launcher $(DESTDIR)$(PREFIX)/bin/elara-launcher

.PHONY: uninstall
uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/elara-launcher
