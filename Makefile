.PHONY: build clean install test

default: build

clean:
	rm -rf Cargo.lock target/

build:
	cargo build --release

install: build
	cp -f target/release/pfsense-dashboard ~/bin/

test:
	cargo test --all
	cargo clippy --all
