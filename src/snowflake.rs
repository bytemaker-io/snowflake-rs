use std::time::Instant;
use std::sync::atomic::{AtomicI64, Ordering};
use std::error::Error;
use std::fmt;

use log::{debug, warn};
use serde::{Deserialize, Serialize};

/// Bit allocation for different parts of the Snowflake ID
const NODE_BITS: u8 = 10;
const STEP_BITS: u8 = 12;
const TIMESTAMP_BITS: u8 = 41;

/// Maximum values for node and step
const NODE_MAX: u16 = (1 << NODE_BITS) - 1;
const STEP_MAX: u16 = (1 << STEP_BITS) - 1;

/// Bit shifting constants
const TIMESTAMP_SHIFT: u8 = NODE_BITS + STEP_BITS;
const NODE_SHIFT: u8 = STEP_BITS;

/// Default epoch (2021-01-01T00:00:00Z in milliseconds since Unix epoch)
const DEFAULT_EPOCH: i64 = 1609459200000;

/// Errors that can occur during Snowflake ID generation
#[derive(Debug,Serialize,Deserialize)]
pub enum SnowflakeError {
    /// Indicates that the system clock has moved backwards
    ClockMovedBackwards,
    /// Indicates that the provided machine ID is out of the valid range
    MachineIdOutOfRange,
    /// Indicates that the sequence number has overflowed
    SequenceOverflow,
}

impl fmt::Display for SnowflakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SnowflakeError::ClockMovedBackwards => write!(f, "Clock moved backwards"),
            SnowflakeError::MachineIdOutOfRange => write!(f, "Machine ID is out of range"),
            SnowflakeError::SequenceOverflow => write!(f, "Sequence overflow"),
        }
    }
}

impl Error for SnowflakeError {}

/// Snowflake ID generator
///
/// This struct implements the Snowflake algorithm for generating unique IDs.
/// Each ID is composed of:
/// - Timestamp (41 bits)
/// - Node ID (10 bits)
/// - Sequence number (12 bits)
pub struct Snowflake {
    node: u16,
    epoch_ms: i64,
    last_timestamp_and_sequence: AtomicI64,
    start: Instant,
}

impl Snowflake {
    /// Creates a new Snowflake instance
    ///
    /// # Arguments
    ///
    /// * `node` - A unique identifier for the node generating the IDs (0-1023)
    /// * `epoch` - An optional custom epoch in milliseconds. If None, DEFAULT_EPOCH is used.
    ///
    /// # Returns
    ///
    /// A Result containing the new Snowflake instance or a SnowflakeError
    ///
    /// # Errors
    ///
    /// Returns SnowflakeError::MachineIdOutOfRange if the node ID is greater than 1023
    pub fn new(node: u16, epoch: Option<i64>) -> Result<Self, SnowflakeError> {
        if node > NODE_MAX {
            return Err(SnowflakeError::MachineIdOutOfRange);
        }
        let epoch_ms = epoch.unwrap_or(DEFAULT_EPOCH);
        Ok(Snowflake {
            node,
            epoch_ms,
            last_timestamp_and_sequence: AtomicI64::new(0),
            start: Instant::now(),
        })
    }

    /// Generates a new Snowflake ID
    ///
    /// # Returns
    ///
    /// A Result containing the generated ID as a u64 or a SnowflakeError
    ///
    /// # Errors
    ///
    /// - SnowflakeError::ClockMovedBackwards if the system time moves backwards
    /// - SnowflakeError::SequenceOverflow if unable to generate a unique ID within 5 seconds
    pub fn generate(&self) -> Result<u64, SnowflakeError> {
        let current_timestamp = self.current_time_millis();
        let mut last_timestamp_and_sequence = self.last_timestamp_and_sequence.load(Ordering::Acquire);

        loop {
            let (last_timestamp, last_sequence) = decode_timestamp_and_sequence(last_timestamp_and_sequence);
            if current_timestamp < last_timestamp {
                return Err(SnowflakeError::ClockMovedBackwards);
            }
            let (new_timestamp, new_sequence) = if current_timestamp == last_timestamp {
                let new_sequence = (last_sequence + 1) & STEP_MAX as i64;
                if new_sequence == 0 {
                    (self.wait_next_millis(last_timestamp)?, 0)
                } else {
                    (current_timestamp, new_sequence)
                }
            } else {
                (current_timestamp, 0)
            };
            let new_timestamp_and_sequence = encode_timestamp_and_sequence(new_timestamp, new_sequence);
            match self.last_timestamp_and_sequence.compare_exchange_weak(
                last_timestamp_and_sequence,
                new_timestamp_and_sequence,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let id = self.create_id(new_timestamp, new_sequence as u16);
                    return Ok(id);
                }
                Err(actual) => {
                    last_timestamp_and_sequence = actual;
                }
            }
        }
    }
    /// Parses a Snowflake ID into its components
    /// # Arguments
    /// * `id` - The Snowflake ID to parse
    /// # Returns
    /// A tuple containing the timestamp, node ID, and sequence number
    /// # Example
    /// ```
    /// let (timestamp, node, sequence) = Snowflake::parse_id(1234567890);
    /// println!("Timestamp: {}, Node: {}, Sequence: {}", timestamp, node, sequence);
    /// ```
    pub fn parse_id(id: u64) -> (u64, u16, u16) {
        let timestamp = (id >> TIMESTAMP_SHIFT) & ((1 << TIMESTAMP_BITS) - 1);
        let node = ((id >> NODE_SHIFT) & ((1 << NODE_BITS) - 1)) as u16;
        let sequence = (id & ((1 << STEP_BITS) - 1)) as u16;
        (timestamp, node, sequence)
    }
    // Waits until the next millisecond
    fn wait_next_millis(&self, last_timestamp: i64) -> Result<i64, SnowflakeError> {
        let start = Instant::now();
        loop {
            let current_timestamp = self.current_time_millis();
            if current_timestamp > last_timestamp {
                return Ok(current_timestamp);
            }
            if start.elapsed().as_millis() > 5000 { // 5 seconds max wait
                return Err(SnowflakeError::SequenceOverflow);
            }
            std::thread::yield_now();
        }
    }

    // Creates the final ID by combining timestamp, node ID, and sequence
    fn create_id(&self, timestamp: i64, sequence: u16) -> u64 {
        (((timestamp - self.epoch_ms) as u64) << TIMESTAMP_SHIFT)
            | ((self.node as u64) << NODE_SHIFT)
            | sequence as u64
    }

    // Returns the current timestamp in milliseconds
    fn current_time_millis(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as i64
    }
}

// Encodes timestamp and sequence into a single i64 value
fn encode_timestamp_and_sequence(timestamp: i64, sequence: i64) -> i64 {
    (timestamp << STEP_BITS) | sequence
}

// Decodes timestamp and sequence from a single i64 value
fn decode_timestamp_and_sequence(value: i64) -> (i64, i64) {
    let timestamp = value >> STEP_BITS;
    let sequence = value & STEP_MAX as i64;
    (timestamp, sequence)
}