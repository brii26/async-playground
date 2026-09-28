// Scenario: tokio::sync::Notify is a simpler, permit-based alternative
// to a Condvar. Key semantic difference from Go's sync.Cond: Notify
// stores AT MOST ONE permit -- notify_one() called before anyone is
// waiting is NOT lost the way Cond.Signal() before Wait() is a no-op in
// Go (in Go, a Signal with no waiter simply does nothing; you must
// already be inside Wait() to receive it, which causes the classic
// "lost wakeup" bug if state isn't checked in a loop under a lock
// first). Notify::notify_one still requires the SAME care for a
// different reason: a permit is only good for one notified() call, so
// mismatched notify/wait counts cause lost wakeups.
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() {
    let notify = Arc::new(Notify::new());

    // notify_one() called with NO waiter yet still stores a permit --
    // the subsequent notified().await consumes it immediately instead
    // of blocking. This differs from Go's Cond.Signal(), which requires
    // the waiter to already be inside Wait().
    notify.notify_one();
    notify.notified().await; // returns immediately, consuming the stored permit
    println!("early notify_one() was not lost -- permit was buffered");

    // Second call: no permit stored, so notified().await actually
    // blocks until another task calls notify_one().
    let notify2 = notify.clone();
    let waiter = tokio::spawn(async move {
        notify2.notified().await;
        println!("waiter woke up from a real notify_one()");
    });
    tokio::time::sleep(Duration::from_millis(10)).await;
    notify.notify_one();
    waiter.await.unwrap();

    // Demonstrate a genuine lost-wakeup risk: notify_one() with two
    // pending waiters only wakes ONE -- the other keeps waiting, same
    // as Go's Cond.Signal() (use notify_waiters() / Broadcast for "wake
    // everyone").
    let notify3 = Arc::new(Notify::new());
    let n1 = notify3.clone();
    let n2 = notify3.clone();
    let w1 = tokio::spawn(async move { n1.notified().await; "w1" });
    let w2 = tokio::spawn(async move { n2.notified().await; "w2" });
    tokio::time::sleep(Duration::from_millis(10)).await;
    notify3.notify_one(); // wakes exactly one of w1/w2
    let woke = timeout(Duration::from_millis(50), async {
        tokio::select! { r = w1 => r.unwrap(), r = w2 => r.unwrap() }
    }).await;
    println!("notify_one with two waiters woke: {woke:?} -- the other is still pending, matching Go's Signal semantics");
}
