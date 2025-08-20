# Markstr Bitcoin Testing Infrastructure

This component provides end-to-end testing infrastructure for Markstr prediction markets using Bitcoin nodes with CSFS (CheckSigFromStack) support.

## Quick Start

```bash
# Build Bitcoin Docker image with CSFS support
cd bitcoind-tests
make image

# Run tests (automatically uses the built image)
cargo test

# Image will be available as: bitcoin/bitcoin-csfs
```

**What this does**: Builds a custom Bitcoin Core Docker image from `jamesob/bitcoin.git` with CSFS opcodes enabled, allowing you to test Markstr prediction markets locally with real Bitcoin transactions.

## Overview

The `bitcoind-tests` crate enables:
- **Automated Testing**: Spawn Bitcoin nodes with CSFS opcodes enabled
- **Docker Integration**: Use containerized Bitcoin Core with custom patches
- **Real Transactions**: Test actual Bitcoin transactions and CSFS script verification
- **Flexible Setup**: Connect to existing nodes or auto-spawn test instances

## Docker Image

### Building the CSFS-Enabled Bitcoin Image

The build process:
1. Clones `willcl-ark/bitcoin-core-docker` for the Dockerfile
2. Modifies it to use `jamesob/bitcoin.git` with CSFS patches
3. Builds Bitcoin Core with CSFS opcodes enabled
4. Creates a Docker image ready for regtest with CSFS support

### Why CSFS is Required

Markstr prediction markets use **CheckSigFromStack (CSFS)** opcodes to verify oracle signatures directly in Bitcoin scripts. Since CSFS is not enabled on Bitcoin mainnet, this custom build provides:

- **CSFS Opcodes**: Enable `OP_CHECKSIGFROMSTACK` for oracle signature verification
- **Regtest Network**: Safe testing environment with instant block generation
- **Transaction Indexing**: Full transaction history for debugging and verification

### Configuration

Default build parameters:
- **Repository**: `https://github.com/jamesob/bitcoin.git`
- **Branch/Commit**: `2025-04-csfs`
- **Image Name**: `bitcoin/bitcoin-csfs`

Override with environment variables to use different Bitcoin repository or specific commit.

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
- **Network Access**: To download Bitcoin Core source

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

**Image build failures**: Verify internet connection for Git clone operations.

**RPC connection timeouts**: Increase wait times in test setup if running on slow hardware.

### Clean Up

Remove temporary Docker containers with `docker container prune`, remove the test Bitcoin image with `docker rmi bitcoin/bitcoin-csfs`, and clean build artifacts with `make clean`.
