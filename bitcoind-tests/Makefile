.PHONY: image clean

REPO_URL := https://github.com/willcl-ark/bitcoin-core-docker
TMP_DIR := /tmp/bitcoin-core-docker
DOCKERFILE := $(TMP_DIR)/master/Dockerfile
BUILD_CONTEXT := $(TMP_DIR)/master
IMAGE_NAME := bitcoin/bitcoin-csfs
BITCOIN_REPO ?= https://github.com/jamesob/bitcoin.git
COMMIT ?= 2025-04-csfs

image:
	@if [ -d "$(TMP_DIR)" ]; then rm -rf "$(TMP_DIR)"; fi
	@git clone --depth 1 "$(REPO_URL)" "$(TMP_DIR)"
	@sed -i '' -E "s|https://github.com/bitcoin/bitcoin(\.git)?|$(BITCOIN_REPO)|g" "$(DOCKERFILE)"
	@docker build -f "$(DOCKERFILE)" --build-arg COMMIT="$(COMMIT)" -t "$(IMAGE_NAME)" "$(BUILD_CONTEXT)"
	@docker tag "$(IMAGE_NAME)" "$(GHCR_IMAGE):$(COMMIT)"
	@docker tag "$(IMAGE_NAME)" "$(GHCR_IMAGE):latest"

clean:
	@rm -rf "$(TMP_DIR)"
