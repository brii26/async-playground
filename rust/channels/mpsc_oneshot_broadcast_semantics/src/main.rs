// Scenario: Go has one channel type with buffered/unbuffered as a
// parameter. Tokio splits channels by USE CASE, each with different
// backpressure and delivery semantics:
//   - mpsc: many producers, one consumer, bounded = backpressure via
//     .send().await blocking when full (like a Go buffered channel).
//   - oneshot: exactly one value, ever -- models a single async reply,
//     zero-cost compared to mpsc for the "return a value from a spawned
//     task" pattern.
//   - broadcast: every subscriber gets every message, but a SLOW
//     subscriber that falls behind the ring buffer capacity gets
//     RecvError::Lagged, silently dropping messages for that consumer
//     only -- there is no Go equivalent; a Go broadcast pattern
//     (fan-out via N channels) would need to be built manually and
//     usually block instead of drop.
use tokio::sync::{broadcast, mpsc, oneshot};

#[tokio::main]
async fn main() {
    // mpsc: bounded backpressure.
    let (tx, mut rx) = mpsc::channel::<i32>(2);
    tokio::spawn(async move {
        for i in 0..5 {
            tx.send(i).await.unwrap(); // blocks once buffer of 2 is full
        }
    });
    let mut got = vec![];
    while let Some(v) = rx.recv().await {
        got.push(v);
    }
    println!("mpsc received: {got:?}");

    // oneshot: single reply, models request/response cleanly.
    let (reply_tx, reply_rx) = oneshot::channel::<&str>();
    tokio::spawn(async move {
        reply_tx.send("computed result").unwrap();
    });
    println!("oneshot reply: {}", reply_rx.await.unwrap());

    // broadcast: slow subscriber lags and drops, does not block sender.
    let (btx, mut fast_rx) = broadcast::channel::<i32>(4);
    let mut slow_rx = btx.subscribe();
    for i in 0..10 {
        btx.send(i).unwrap(); // never blocks, even though slow_rx hasn't read yet
    }
    let mut fast_count = 0;
    while let Ok(_) = fast_rx.try_recv() {
        fast_count += 1;
    }
    println!("fast subscriber drained: {fast_count} messages");
    match slow_rx.recv().await {
        Ok(v) => println!("slow subscriber got: {v}"),
        Err(broadcast::error::RecvError::Lagged(n)) => {
            println!("slow subscriber LAGGED and dropped {n} messages -- broadcast never blocks the sender for a slow reader")
        }
        Err(e) => println!("error: {e}"),
    }
}
