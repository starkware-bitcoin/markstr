# Markstr Bitcoin Testing Infrastructure

This component provides end-to-end testing infrastructure for Markstr prediction markets using Bitcoin nodes with CSFS (CheckSigFromStack) support.

## Quick Start

```bash
# Pull prebuilt CSFS-enabled Bitcoin Core image from GHCR
docker pull ghcr.io/AbdelStark/bitcoin-csfs:latest

# Retag to the name expected by the tests (optional)
docker tag ghcr.io/AbdelStark/bitcoin-csfs:latest bitcoin/bitcoin-csfs

# Run tests (automatically uses the image)
cargo test -p bitcoind-tests
```

**What this does**: Uses a custom Bitcoin Core Docker image with CSFS opcodes enabled, allowing you to test Markstr prediction markets locally with real Bitcoin transactions.

## Overview

The `bitcoind-tests` crate enables:
- **Automated Testing**: Spawn Bitcoin nodes with CSFS opcodes enabled
- **Docker Integration**: Use containerized Bitcoin Core with custom patches
- **Real Transactions**: Test actual Bitcoin transactions and CSFS script verification
- **Flexible Setup**: Connect to existing nodes or auto-spawn test instances

## Docker Image

### Prebuilt Image

The image is published to GitHub Container Registry (multi-arch: linux/amd64, linux/arm64):
- `ghcr.io/AbdelStark/bitcoin-csfs:latest`
- `ghcr.io/AbdelStark/bitcoin-csfs:<commit>` (e.g., `2025-04-csfs`)

You can use it directly, or retag it locally to `bitcoin/bitcoin-csfs` to match the default test harness image name.

### Building Locally (optional)

A `Dockerfile` is provided in this folder if you prefer local builds:

```bash
cd bitcoind-tests
# Build from a specific Bitcoin repo/commit (defaults shown)
docker build \
  --build-arg BITCOIN_REPO=https://github.com/jamesob/bitcoin.git \
  --build-arg COMMIT=2025-04-csfs \
  -t bitcoin/bitcoin-csfs .
```

### Why CSFS is Required

Markstr prediction markets use **CheckSigFromStack (CSFS)** opcodes to verify oracle signatures directly in Bitcoin scripts. Since CSFS is not enabled on Bitcoin mainnet, this custom build provides:

- **CSFS Opcodes**: Enable `OP_CHECKSIGFROMSTACK` for oracle signature verification
- **Regtest Network**: Safe testing environment with instant block generation
- **Transaction Indexing**: Full transaction history for debugging and verification

### Configuration

Default build parameters:
- **Repository**: `https://github.com/jamesob/bitcoin.git`
- **Branch/Commit**: `2025-04-csfs`
- **Image Name**: `ghcr.io/AbdelStark/bitcoin-csfs` (retag locally to `bitcoin/bitcoin-csfs` if needed)

Override with build arguments to use a different Bitcoin repository or specific commit when building locally.

## Usage

### Automatic Test Setup

The `TestNode::start()` method automatically starts a Docker Bitcoin node with CSFS and creates a funded wallet for testing.

### Manual Node Connection

Set environment variables to use an existing node:
- `BITCOIN_RPC_URL` for the RPC endpoint
- `BITCOIN_RPC_USER` and `BITCOIN_RPC_PASS` for authentication
- Or `BITCOIN_COOKIE_FILE` for cookie authentication

### Running Tests

Run tests with `cargo test -p bitcoind-tests`. Use `RUST_LOG=debug` for detailed logging.

## Architecture

### TestNode Structure

The `TestNode` struct provides a Bitcoin RPC client, optional Docker process handle, and funded wallet name.

### Docker Process Management

The `DockerBitcoind` struct manages the Docker container process, RPC endpoint URL, authentication cookie, and temporary Bitcoin data directory.

## Requirements

### System Dependencies

- **Docker**: For containerized Bitcoin nodes
- **Rust**: Compilation and test execution
- **Network Access**: To download Bitcoin Core source (local builds only)

### Docker Configuration

The Bitcoin container runs with:
- **Network**: Regtest mode
- **RPC**: Enabled on all interfaces
- **Ports**: Auto-detected available port
- **Data**: Temporary directory (cleaned up after tests)
- **Features**: CSFS opcodes enabled, transaction indexing

## Development

### Adding New Tests

Create test functions that use `TestNode::start()` to get a funded Bitcoin node with CSFS support. The node provides an RPC client for interacting with Bitcoin Core.

### Debugging

Enable detailed logging with `RUST_LOG=bitcoind_tests=debug,bitcoincore_rpc=debug cargo test`.

Check Docker container status with `docker ps` and `docker logs <container_id>`.

## Troubleshooting

### Common Issues

**Port conflicts**: The test framework automatically finds available ports, but ensure Docker can bind to them.

**Permission errors**: Ensure Docker daemon is running and accessible.

**Image issues**: If the image is missing, either pull from GHCR (recommended) or build locally via the provided `Dockerfile`.

**RPC connection timeouts**: Increase wait times in test setup if running on slow hardware.

### Clean Up

Remove temporary Docker containers with `docker container prune`, remove the test Bitcoin image with `docker rmi bitcoin/bitcoin-csfs`, and clean build artifacts with `cargo clean`.
