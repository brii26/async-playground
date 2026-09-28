// Scenario: tokio::sync::Semaphore permits are RAII guards (SemaphorePermit)
// -- dropping the guard releases the permit automatically, including on
// panic or early return, which a hand-rolled acquire/release pair
// (like the Go channel-token pattern) does NOT get for free. The
// footgun: `forget()`-ing a permit (or storing it in a struct that
// outlives its intended scope) silently reduces total concurrency
// forever, because nothing will ever release it. Also: tokio's
// Semaphore IS strictly FIFO-fair among waiters -- unlike a bare
// channel-based semaphore, which only approximates FIFO.
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn faulty_leak_permit(sem: Arc<Semaphore>) {
    let permit = sem.acquire().await.unwrap();
    // BUG: forgetting the permit means it is never released, even
    // though the function returns normally -- this permanently reduces
    // available concurrency by one for the life of the semaphore.
    std::mem::forget(permit);
}

async fn correct_scoped_permit(sem: Arc<Semaphore>, id: u32) {
    let _permit = sem.acquire().await.unwrap(); // released on drop at scope end
    println!("task {id} holds a permit");
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
} // _permit dropped here -> permit returned to the semaphore, even on panic unwind

#[tokio::main]
async fn main() {
    let sem = Arc::new(Semaphore::new(2));

    faulty_leak_permit(sem.clone()).await;
    println!("available permits after leak: {}", sem.available_permits()); // 1, not 2 -- permanently down one

    let mut handles = vec![];
    for i in 0..4 {
        let sem = sem.clone();
        handles.push(tokio::spawn(correct_scoped_permit(sem, i)));
    }
    for h in handles {
        h.await.unwrap();
    }
    println!("available permits after correct usage completes: {}", sem.available_permits());
    println!("takeaway: RAII permits self-release on panic/early-return; mem::forget or storing them too long silently shrinks capacity");
}
