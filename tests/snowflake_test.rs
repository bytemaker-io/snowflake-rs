use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use std::time::{Duration, Instant};
use parking_lot::Mutex;
use snowflake::snowflake::Snowflake;

/// Test the number of IDs generated per second using a single thread
#[test]
fn test_ids_per_second_single_thread() {
    // Create a new Snowflake instance
    let snowflake = Snowflake::new(1, None).unwrap();
    let duration = Duration::from_secs(1);

    let start = Instant::now();
    let mut count = 0;
    // Generate IDs for one second
    while start.elapsed() < duration {
        if snowflake.generate().is_ok() {
            count += 1;
        }
    }

    println!("Generated {} IDs per second with a single thread", count);
    assert!(count > 0);
}

/// Test the number of IDs generated per second using multiple threads
#[test]
fn test_ids_per_second() {
    let snowflake = Arc::new(Snowflake::new(1, None).unwrap());
    let duration = Duration::from_secs(1);
    let thread_count = 4;

    // Spawn multiple threads to generate IDs
    let handles: Vec<_> = (0..thread_count)
        .map(|_| {
            let snowflake = Arc::clone(&snowflake);
            thread::spawn(move || {
                let mut count = 0;
                let start = Instant::now();
                while start.elapsed() < duration {
                    if snowflake.generate().is_ok() {
                        count += 1;
                    }
                }
                count
            })
        })
        .collect();

    // Sum up the total number of IDs generated across all threads
    let total_ids: usize = handles.into_iter()
        .map(|h| h.join().unwrap())
        .sum();

    println!("Generated {} IDs per second with {} threads", total_ids, thread_count);
    assert!(total_ids > 0);
}

/// Test the uniqueness of generated IDs
#[test]
fn test_unique_ids() {
    let snowflake = Arc::new(Snowflake::new(1, None).unwrap());
    let mut handles = vec![];
    let ids = Arc::new(Mutex::new(HashSet::new()));
    let errors = Arc::new(AtomicUsize::new(0));
    let duplicates = Arc::new(AtomicUsize::new(0));

    // Spawn 10 threads, each generating 1000 IDs
    for _ in 0..10 {
        let snowflake = Arc::clone(&snowflake);
        let ids = Arc::clone(&ids);
        let errors = Arc::clone(&errors);
        let duplicates = Arc::clone(&duplicates);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                match snowflake.generate() {
                    Ok(id) => {
                        let mut ids_lock = ids.lock();
                        if !ids_lock.insert(id) {
                            duplicates.fetch_add(1, Ordering::SeqCst);
                            println!("Duplicate ID detected: {}", id);
                        }
                    },
                    Err(_) => {
                        errors.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        }));
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Check results
    let unique_ids = ids.lock().len();
    let error_count = errors.load(Ordering::SeqCst);
    let duplicate_count = duplicates.load(Ordering::SeqCst);
    println!("Unique IDs: {}, Errors: {}, Duplicates: {}", unique_ids, error_count, duplicate_count);
    assert_eq!(unique_ids + error_count, 10000);
    assert_eq!(duplicate_count, 0, "Duplicate IDs detected");
}

/// Test that creating a Snowflake with a node ID out of range returns an error
#[test]
fn test_node_out_of_range() {
    assert!(Snowflake::new(1024,None).is_err());
}