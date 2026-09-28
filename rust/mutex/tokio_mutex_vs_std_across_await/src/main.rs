// Scenario: std::sync::MutexGuard is NOT Send across .await points (as
// of current rustc, holding a std Mutex guard across an await either
// fails to compile under certain futures or, if it does compile because
// the guard is dropped before the await, silently doesn't protect what
// you think it does). tokio::sync::Mutex exists specifically to be held
// across .await -- but that convenience has a cost: it's an async lock
// implemented via a wait queue, meaningfully slower than std::sync::Mutex
// for the common case of short, sync-only critical sections. Rule of
// thumb: use std::sync::Mutex and keep the critical section free of
// .await; reach for tokio::sync::Mutex only when the critical section
// must itself await (e.g. because it wraps an async resource).

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{sleep, Duration};

async fn with_std_mutex(counter: Arc<StdMutex<i64>>) {
    // Correct pattern: scope the guard so it's dropped BEFORE any await.
    {
        let mut guard = counter.lock().unwrap();
        *guard += 1;
    } // guard dropped here, safe to await next
    sleep(Duration::from_millis(1)).await;
}

async fn with_tokio_mutex(counter: Arc<TokioMutex<i64>>) {
    // tokio::sync::Mutex's guard IS Send, so this compiles even though
    // the guard is live across the await -- but now every other task
    // waiting on this lock is blocked for the FULL duration including
    // the sleep, not just the increment. This is the async analogue of
    // "holding a lock across I/O" and is usually a design smell.
    let mut guard = counter.lock().await;
    *guard += 1;
    sleep(Duration::from_millis(1)).await; // BAD: lock held across await
}

#[tokio::main]
async fn main() {
    let std_counter = Arc::new(StdMutex::new(0));
    let mut handles = vec![];
    let start = std::time::Instant::now();
    for _ in 0..50 {
        let c = std_counter.clone();
        handles.push(tokio::spawn(with_std_mutex(c)));
    }
    for h in handles {
        h.await.unwrap();
    }
    println!("std::sync::Mutex (short critical section): {:?}", start.elapsed());

    let tokio_counter = Arc::new(TokioMutex::new(0));
    let mut handles = vec![];
    let start = std::time::Instant::now();
    for _ in 0..50 {
        let c = tokio_counter.clone();
        handles.push(tokio::spawn(with_tokio_mutex(c)));
    }
    for h in handles {
        h.await.unwrap();
    }
    println!("tokio::sync::Mutex (lock held across await): {:?}", start.elapsed());
}
