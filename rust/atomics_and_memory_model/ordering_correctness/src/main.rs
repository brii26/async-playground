// Scenario: Rust exposes the C++11 memory model explicitly via
// std::sync::atomic::Ordering -- Go's atomic package hides this and
// always behaves as SeqCst. Using Relaxed where you need Acquire/Release
// is a real, reproducible-under-load bug: it allows the compiler/CPU to
// reorder the surrounding non-atomic memory accesses, so a Relaxed
// "ready" flag can be observed true by another thread BEFORE the data
// it's supposed to guard is actually visible to that thread.
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;

struct Shared {
    data: AtomicI64,
    ready: AtomicBool,
}

fn broken_relaxed(shared: Arc<Shared>) {
    let writer = {
        let shared = shared.clone();
        thread::spawn(move || {
            shared.data.store(42, Ordering::Relaxed);
            // BUG: Relaxed gives no ordering guarantee relative to the
            // data store above -- a reader thread can observe
            // ready==true while still seeing data's OLD value, because
            // nothing here prevents the reordering (in practice, on
            // x86 this rarely reproduces due to a strong hardware
            // model, but it's a real bug on ARM and is UB per the Rust
            // memory model regardless of what any one CPU does today).
            shared.ready.store(true, Ordering::Relaxed);
        })
    };
    writer.join().unwrap();
}

fn correct_acquire_release(shared: Arc<Shared>) {
    let writer = {
        let shared = shared.clone();
        thread::spawn(move || {
            shared.data.store(42, Ordering::Relaxed); // data itself can stay Relaxed
            // Release here creates a happens-before edge: any thread
            // that observes this store with Acquire is guaranteed to
            // see the data.store above too.
            shared.ready.store(true, Ordering::Release);
        })
    };
    writer.join().unwrap();

    // A correct reader would use: shared.ready.load(Ordering::Acquire)
    // and only then trust shared.data.load(Ordering::Relaxed).
    let _ = shared.data.load(Ordering::Acquire);
}

fn main() {
    let shared = Arc::new(Shared {
        data: AtomicI64::new(0),
        ready: AtomicBool::new(false),
    });
    broken_relaxed(shared.clone());
    correct_acquire_release(shared);
    println!("takeaway: Ordering::Relaxed has no synchronizes-with edge -- pair flag+payload with Release (writer) / Acquire (reader), don't default to Relaxed for 'it compiled and passed once'");
}
