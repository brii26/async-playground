// Bounded mpsc channel IS the backpressure mechanism: send().await
// suspends the producer task (not blocks an OS thread) once the
// channel is full, until a worker frees capacity by receiving. This is
// strictly better than Go's equivalent (a buffered channel blocking a
// whole OS thread/goroutine's stack) only in that it composes with
// select! for cancellable submission -- semantically it's the same
// backpressure idea.
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<i32>(3); // small buffer forces backpressure

    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            tokio::time::sleep(Duration::from_millis(20)).await; // slow worker
            println!("processed {job}");
        }
    });

    for i in 0..8 {
        // Cancellable submission: give up if backpressure persists past a deadline.
        match timeout(Duration::from_millis(200), tx.send(i)).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => println!("channel closed"),
            Err(_) => println!("submit {i} timed out waiting for backpressure to clear"),
        }
    }
}
