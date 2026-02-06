.PHONY: run dev build release clean test

run:
	@if [ -f .env ]; then set -a && . ./.env && set +a; fi && cargo run

dev:
	@if [ -f .env ]; then set -a && . ./.env && set +a; fi && cargo run

build:
	cargo build

release:
	cargo build --release

clean:
	cargo clean

test:
	cargo test
