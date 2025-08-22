# syntax=docker/dockerfile:1

FROM debian:bookworm

ARG DEBIAN_FRONTEND=noninteractive

# Build arguments (can be overridden at build time)
ARG BITCOIN_REPO=https://github.com/jamesob/bitcoin.git
ARG COMMIT=2025-04-csfs

# System and build dependencies
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		ca-certificates \
		curl \
		git \
		build-essential \
		libtool \
		autotools-dev \
		automake \
		pkg-config \
		bsdmainutils \
		python3 \
		libevent-dev \
		libboost-system-dev \
		libboost-filesystem-dev \
		libboost-test-dev \
		libboost-thread-dev \
		libsqlite3-dev \
		libzmq3-dev \
	&& rm -rf /var/lib/apt/lists/*

# Fetch and build Bitcoin Core with CSFS support from the specified repo/commit
WORKDIR /src
RUN git clone "$BITCOIN_REPO" bitcoin \
	&& cd bitcoin \
	&& git fetch --all --tags \
	&& git checkout "$COMMIT" \
	&& ./autogen.sh \
	&& ./configure \
		--without-gui \
		--disable-bench \
		--disable-tests \
	&& make -j"$(nproc)" \
	&& make install

# Create runtime user and data directory expected by tests
RUN useradd -ms /bin/bash bitcoin \
	&& mkdir -p /home/bitcoin/.bitcoin \
	&& chown -R bitcoin:bitcoin /home/bitcoin

ENV HOME=/home/bitcoin
WORKDIR /home/bitcoin

# Default command (tests override and pass flags explicitly)
CMD ["bitcoind", "-regtest"]