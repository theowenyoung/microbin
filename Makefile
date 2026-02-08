DOCKER_IMAGE := owenyoung/microbin
DOCKER_TAG := latest
DOCKER_PLATFORMS := linux/amd64,linux/arm64

.PHONY: run dev build release clean test docker-push tag patch

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

# Bump the fork revision (e.g. 2.1.0-1 -> 2.1.0-2), update lock file, and commit.
patch:
	$(eval CURRENT := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'))
	$(eval BASE := $(shell echo $(CURRENT) | sed 's/-[0-9]*$$//'))
	$(eval REV := $(shell echo $(CURRENT) | sed 's/.*-//'))
	$(eval NEXT := $(BASE)-$(shell echo $$(($(REV) + 1))))
	@echo "Bumping version: $(CURRENT) -> $(NEXT)"
	perl -i -pe 'if (!$$done && s/^version = ".*"/version = "$(NEXT)"/) { $$done = 1 }' Cargo.toml
	cargo check --quiet
	git add Cargo.toml Cargo.lock
	git commit -m "v$(NEXT)"

# Create a git tag and push it to trigger the CI release workflow.
#
# Usage:
#   make tag                # tag with version from Cargo.toml (e.g. v2.1.0)
#   make tag v=2.2.0        # tag with a specific version
tag:
	$(eval v ?= $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'))
	@echo "Tagging v$(v) and pushing to origin..."
	git tag v$(v)
	git push origin v$(v)
