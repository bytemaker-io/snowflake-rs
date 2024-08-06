# Snowflake ID Generator

A Rust implementation of the Snowflake ID generator, which produces unique 64-bit IDs. This implementation ensures thread safety and high performance, suitable for distributed systems.

## Overview

Each Snowflake ID consists of three parts:
- **Timestamp**: 41 bits
- **Node ID**: 10 bits
- **Sequence Number**: 12 bits

### Bit Allocation

- **Timestamp (41 bits)**: Milliseconds since a custom epoch.
- **Node ID (10 bits)**: Unique identifier for the node generating the IDs (0-1023).
- **Sequence Number (12 bits)**: Incremental counter within the same millisecond.

## Features

- **Thread-safe**: Can be used safely across multiple threads.
- **Custom Epoch**: Allows setting a custom epoch.
- **High Performance**: Generates a large number of IDs per second.

## Usage

### Add to Cargo.toml

```toml
[dependencies]
snowflake-rs-impl="*"
```
## Example

```rust
use snowflake::Snowflake;

fn main() {
    // Create a new Snowflake instance with node ID 1 and default epoch
    let snowflake = Snowflake::new(1, None).unwrap();

    // Generate a new ID
    let id = snowflake.generate().unwrap();
    println!("Generated ID: {}", id);
}
```

## Custom Epoch Example

```rust
use snowflake::Snowflake;

fn main() {
    // Custom epoch (2023-01-01T00:00:00Z in milliseconds since Unix epoch)
    let custom_epoch = 1672531200000;
    let snowflake = Snowflake::new(1, Some(custom_epoch)).unwrap();

    // Generate a new ID
    let id = snowflake.generate().unwrap();
    println!("Generated ID: {}", id);
}
```

## Testing
This library includes tests to verify the correct functionality of the Snowflake ID generator.
### Run Tests
```rust
cargo test
```
### Included Tests
- Single-threaded ID generation: Measures IDs generated per second in a single thread.
- Multi-threaded ID generation: Measures IDs generated per second using multiple threads.
- Uniqueness: Ensures that all generated IDs are unique.
- Node ID range validation: Verifies that creating a Snowflake with an invalid node ID returns an error.

## Benchmarking
```rust
cargo bench
```
Approximately 4,100,000 IDs per second
