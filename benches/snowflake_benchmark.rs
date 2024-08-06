use criterion::{black_box, criterion_group, criterion_main, Criterion};
use snowflake_rs_impl::snowflake::Snowflake;
use std::sync::Arc;
use std::thread;

fn benchmark_single_thread(c: &mut Criterion) {
    let snowflake = Snowflake::new(1, None).unwrap();
    c.bench_function("single thread generation", |b| {
        b.iter(|| {
            black_box(snowflake.generate().unwrap());
        })
    });
}

fn benchmark_multi_thread(c: &mut Criterion) {
    c.bench_function("multi thread generation", |b| {
        b.iter(|| {
            let snowflake = Arc::new(Snowflake::new(1, None).unwrap());
            let mut handles = vec![];
            for _ in 0..4 {
                let sf = Arc::clone(&snowflake);
                handles.push(thread::spawn(move || {
                    for _ in 0..250 {
                        black_box(sf.generate().unwrap());
                    }
                }));
            }
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

criterion_group!(benches, benchmark_single_thread, benchmark_multi_thread);
criterion_main!(benches);