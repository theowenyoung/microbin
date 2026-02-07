DOCKER_IMAGE := owenyoung/microbin
DOCKER_TAG := latest
DOCKER_PLATFORMS := linux/amd64,linux/arm64

.PHONY: run dev build release clean test docker-push

# Run the dev server (loads .env)
run:
	@if [ -f .env ]; then set -a && . ./.env && set +a; fi && cargo run

# Same as run
dev:
	@if [ -f .env ]; then set -a && . ./.env && set +a; fi && cargo run

# Debug build
build:
	cargo build

# Release build (LTO enabled, stripped)
release:
	cargo build --release

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test

# Build and push multi-platform Docker image to Docker Hub.
# Requires `docker login` first.
#
# Usage:
#   make docker-push                       # push owenyoung/microbin:latest
#   make docker-push DOCKER_TAG=v1.0.0     # push with a specific tag
docker-push:
	docker buildx build --platform $(DOCKER_PLATFORMS) \
		-t $(DOCKER_IMAGE):$(DOCKER_TAG) \
		-f Dockerfile.prod --push .
