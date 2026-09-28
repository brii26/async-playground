// Scenario: tokio::spawn returns a JoinHandle; dropping the handle does
// NOT cancel the task (unlike a scoped/structured-concurrency API) --
// the task keeps running detached from anything that could stop it.
// This is the Rust analogue of Go's "forgot to cancel via context"
// leak, but with a twist: in Rust, if the task loops forever awaiting
// a channel that will never receive another message, dropping the
// SENDER is what naturally ends it (recv() returns None) -- so the fix
// is often about lifetime/ownership of channel halves, not about
// explicit cancellation tokens.
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

async fn leaky_spawn() {
    let (tx, mut rx) = mpsc::channel::<i32>(1);
    // tx is captured into the task below and never dropped by the
    // caller of leaky_spawn -- wait, actually the bug here is that we
    // spawn a receiver loop but hold `tx` in this outer scope without
    // ever sending or dropping it, so rx.recv() blocks forever and the
    // spawned task is never reachable again once this function returns.
    tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            println!("received {v}");
        }
        println!("this line never runs -- tx above is never dropped, so recv() never sees None");
    });
    std::mem::forget(tx); // simulates tx being leaked into a long-lived struct
    // that nobody ever drops -- the task above is now permanently
    // parked awaiting a message or channel closure that will never come.
}

async fn fixed_spawn() {
    let (tx, mut rx) = mpsc::channel::<i32>(1);
    let handle = tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            println!("received {v}");
        }
        println!("channel closed cleanly, task exits");
    });
    tx.send(1).await.unwrap();
    drop(tx); // dropping the sender is what lets recv() return None and the task end
    handle.await.unwrap();
}

#[tokio::main]
async fn main() {
    leaky_spawn().await;
    sleep(Duration::from_millis(20)).await;
    println!("leaky_spawn's task is still parked forever (detected only via tokio-console/task count in real diagnostics)");

    fixed_spawn().await;
    println!("takeaway: an mpsc receiver loop ends only when ALL senders are dropped -- track sender lifetime as carefully as the JoinHandle itself");
}
