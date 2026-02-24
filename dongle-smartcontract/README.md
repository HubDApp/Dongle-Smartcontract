# Dongle Smart Contract

A Soroban smart contract for decentralized project registry, reviews, and verification on the Stellar network.

## Quick Start

### Prerequisites

- Rust 1.74.0+
- Soroban CLI
- wasm32-unknown-unknown target

### Install Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli --features opt
```

### Build

```bash
make build
# or
cargo build --target wasm32-unknown-unknown --release
```

### Test

```bash
make test
# or
cargo test
```

### Deploy to Testnet

```bash
# Configure network (first time only)
soroban network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Create identity (first time only)
soroban keys generate --global alice --network testnet

# Fund account (first time only)
soroban keys fund alice --network testnet

# Deploy
make deploy-testnet
```

## Features

- **Project Registry**: Register and manage project metadata on-chain
- **Review System**: Submit and manage project reviews with ratings
- **Verification**: Request and approve project verification
- **Fee Management**: Configurable fees for operations
- **Access Control**: Owner-based permissions

## Contract Functions

### Project Management

- `register_project` - Register a new project
- `update_project` - Update project metadata (owner only)
- `get_project` - Retrieve project details
- `list_projects` - List all projects

### Reviews

- `add_review` - Submit a project review
- `update_review` - Update your review
- `get_review` - Get a specific review
- `get_project_reviews` - List project reviews

### Verification

- `request_verification` - Request project verification
- `approve_verification` - Approve verification (admin only)
- `reject_verification` - Reject verification (admin only)
- `get_verification` - Get verification status

### Administration

- `initialize` - Initialize contract with admin
- `set_admin` - Update admin address
- `set_fee_config` - Configure fees
- `set_treasury` - Set treasury address

## Development

### Using Makefile

```bash
make help           # Show all commands
make build          # Build contract
make test           # Run tests
make test-verbose   # Run tests with output
make fmt            # Format code
make lint           # Run linter
make clean          # Clean artifacts
make dev            # Run full dev workflow
```

### Manual Commands

```bash
# Build
cargo build --target wasm32-unknown-unknown --release

# Test
cargo test

# Format
cargo fmt

# Lint
cargo clippy

# Clean
cargo clean
```

## Testing

The contract includes comprehensive tests covering:

- Project registration and validation
- Ownership and authorization
- Review submission and updates
- Verification workflow
- Fee management
- Edge cases and error handling

Run tests:
```bash
cargo test
```

Run specific test:
```bash
cargo test test_register_project_success
```

## Documentation

See [SETUP.md](../SETUP.md) for detailed setup, deployment, and usage instructions.

## Project Structure

```
src/
├── lib.rs                    # Main contract interface
├── constants.rs              # Constants and limits
├── errors.rs                 # Error definitions
├── events.rs                 # Event emissions
├── fee_manager.rs            # Fee handling
├── project_registry.rs       # Project management
├── review_registry.rs        # Review system
├── verification_registry.rs  # Verification logic
├── rating_calculator.rs      # Rating calculations
├── storage_keys.rs           # Storage keys
├── types.rs                  # Data structures
├── utils.rs                  # Utilities
└── test.rs                   # Tests
```

## License

See [LICENSE](../LICENSE) file for details.

## Contributing

Contributions are welcome! Please open an issue or pull request.

## Resources

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Developer Portal](https://developers.stellar.org/)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
