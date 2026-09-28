// Scenario: tokio::select! drops all non-winning futures -- if a
// branch's future had already made partial, observable progress
// (e.g. partially written to a buffer, or holds a lock it hasn't yet
// released) before being dropped, that progress is LOST or, worse,
// left in an inconsistent state. This is "cancellation safety": a
// future is cancellation-safe only if being dropped at ANY .await
// point inside it leaves no partial side effects. tokio documents this
// per-method (e.g. AsyncReadExt::read is cancel-safe; a loop that reads
// partial data into a shared Vec across multiple awaits mid-loop is
// NOT, unless designed for it). Go's context cancellation has no
// direct analogue to this hazard because Go cancellation is purely
// advisory/observed via ctx.Done(), never "drop your future mid-flight"
// -- so this is a genuinely new failure class for engineers new to Rust
// async coming from Go.
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use std::sync::Arc;

async fn not_cancel_safe(counter: Arc<Mutex<i32>>) {
    let mut guard = counter.lock().await;
    *guard += 1; // step 1 of a logically-two-step update
    sleep(Duration::from_millis(50)).await; // if select! cancels HERE,
    // the lock guard is dropped (releasing the mutex, fine) but the
    // "logical operation" this function represents is left half-done:
    // the counter was incremented but whatever this function was
    // ALSO supposed to do (e.g. write a matching log entry) never runs.
    println!("second half of update completed, counter={}", *guard);
}

#[tokio::main]
async fn main() {
    let counter = Arc::new(Mutex::new(0));

    tokio::select! {
        _ = not_cancel_safe(counter.clone()) => {
            println!("update completed normally");
        }
        _ = sleep(Duration::from_millis(10)) => {
            // This branch wins the race -- not_cancel_safe's future is
            // dropped mid-await, after step 1 but before step 2.
            println!("select! timed out and dropped the update future mid-flight");
        }
    }

    let final_val = *counter.lock().await;
    println!("counter ended at {final_val} (step 1 ran, step 2 -- the log line -- never did)");
    println!("takeaway: design futures used inside select! to be atomic per .await point, or make partial state explicitly recoverable");
}
