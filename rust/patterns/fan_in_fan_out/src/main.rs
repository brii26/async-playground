// Fan-out/fan-in with mpsc: N workers share one input mpsc receiver
// (wrapped so only one worker consumes each item -- mpsc naturally
// distributes across concurrent recv() callers when wrapped in a
// shared Mutex, since tokio's mpsc::Receiver itself is not Clone).
// Fan-in is simpler than Go here: all workers send into the SAME mpsc
// sender clone, so results merge automatically -- no manual WaitGroup
// + closer-goroutine needed. The channel closes once all Sender clones
// drop, which happens automatically when the worker tasks finish.
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[tokio::main]
async fn main() {
    let (work_tx, work_rx) = mpsc::channel::<i32>(100);
    let work_rx = Arc::new(Mutex::new(work_rx));
    let (result_tx, mut result_rx) = mpsc::channel::<i32>(100);

    for _ in 0..4 {
        let work_rx = work_rx.clone();
        let result_tx = result_tx.clone();
        tokio::spawn(async move {
            loop {
                let item = { work_rx.lock().await.recv().await };
                match item {
                    Some(v) => result_tx.send(v * v).await.unwrap(),
                    None => break, // input closed and drained
                }
            }
        }); // result_tx clone drops here when the task ends
    }
    drop(result_tx); // drop our own extra clone so the channel can close once workers finish

    for i in 1..=20 {
        work_tx.send(i).await.unwrap();
    }
    drop(work_tx); // signals workers that no more work is coming

    let mut sum = 0;
    while let Some(v) = result_rx.recv().await {
        sum += v;
    }
    println!("sum of squares: {sum}");
}
