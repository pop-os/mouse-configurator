prefix ?= /usr
sysconfdir ?= /etc
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

SRC = Cargo.toml Cargo.lock Makefile $(shell find src -type f -wholename '*src/*.rs')

.PHONY: all clean distclean install uninstall update

APPID = "com.hp.mouse-configurator"
BIN=hp-mouse-configurator
DESKTOP = $(APPID).desktop
ICON = $(APPID).svg

TARGET = debug
DEBUG ?= 0
ifeq ($(DEBUG),0)
	TARGET = release
	ARGS += --release
endif

VENDOR ?= 0
ifneq ($(VENDOR),0)
	ARGS += --frozen
endif

all: target/release/$(BIN)

clean:
	cargo clean

distclean:
	rm -rf .cargo vendor vendor.tar.xz

install: all
	install -D -m 0755 "target/release/$(BIN)" "$(DESTDIR)$(bindir)/$(BIN)"
	install -Dm0644 "data/$(DESKTOP)" "$(DESTDIR)$(datadir)/applications/$(DESKTOP)"
	install -Dm0644 "data/$(ICON)" "$(DESTDIR)$(datadir)/icons/hicolor/scalable/apps/$(ICON)"

uninstall:
	rm -f "$(DESTDIR)$(bindir)/$(BIN)"
	rm -f "$(DESTDIR)$(datadir)/applications/$(DESKTOP)"
	rm -f "$(DESTDIR)$(datadir)/icons/hicolor/scalable/apps/$(ICON)"

update:
	cargo update

vendor:
	rm .cargo -rf
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar cf vendor.tar vendor
	rm -rf vendor

vendor-check:
ifeq ($(VENDOR),1)
	rm vendor -rf && tar xf vendor.tar
endif

target/release/$(BIN): $(SRC) vendor-check
	cargo build $(ARGS)

	cargo build $(ARGS)
