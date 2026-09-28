// Scenario: tokio::task::spawn_blocking moves a closure to a dedicated
// blocking-thread pool (grows up to 512 threads by default), returning
// a JoinHandle you await -- the correct way to run CPU-bound or
// syscall-blocking work from async code without starving the async
// worker threads. tokio::task::block_in_place is a DIFFERENT tool: it
// only works on the multi_thread runtime, and it tells the CURRENT
// worker thread "I'm about to block, hand my other queued tasks off to
// another worker" -- it doesn't move the work anywhere, it just warns
// the runtime. Using block_in_place on current_thread panics at
// runtime; using spawn_blocking is almost always the safer default.
use std::time::Duration;
use tokio::task;

fn blocking_work() -> u64 {
    std::thread::sleep(Duration::from_millis(50)); // simulate a blocking syscall
    42
}

#[tokio::main]
async fn main() {
    // Correct: offload to the blocking pool, async worker threads stay
    // free to run other tasks the whole time.
    let handle = task::spawn_blocking(blocking_work);
    let other = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(5)).await;
        println!("this async task ran concurrently with the blocking work");
    });
    let (result, _) = tokio::join!(handle, other);
    println!("spawn_blocking result: {:?}", result.unwrap());

    // block_in_place: warns the runtime this worker thread is about to
    // block, so it can migrate other queued tasks off first. Only
    // valid on multi_thread runtime; requires #[tokio::main] default
    // (which IS multi_thread) here.
    let result = task::block_in_place(blocking_work);
    println!("block_in_place result: {result}");
    println!("takeaway: spawn_blocking moves work off the async pool entirely; block_in_place blocks the CURRENT worker but signals the runtime to rebalance the rest of its queue");
}
