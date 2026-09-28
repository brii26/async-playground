// Scenario: tokio_util::sync::CancellationToken is the idiomatic
// equivalent of Go's context.Context for shutdown signaling, but it's
// a tree of tokens (child_token()) rather than a chained-context
// value bag. Same shutdown ordering rule as Go applies: stop accepting
// new work, THEN wait for in-flight to drain, bounded by a deadline so
// a stuck task can't block shutdown forever. Difference worth noting:
// a token has no equivalent to Go's context.WithValue -- cancellation
// and request-scoped values are deliberately separate concerns in the
// Rust ecosystem, whereas Go's Context conflates them (a known API
// design critique of Go's context package).
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout, Duration};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let root_token = CancellationToken::new();
    let accepting = Arc::new(AtomicBool::new(true));
    let mut in_flight: JoinSet<()> = JoinSet::new();

    for i in 0..5 {
        if !accepting.load(Ordering::SeqCst) {
            break;
        }
        let child = root_token.child_token();
        in_flight.spawn(async move {
            tokio::select! {
                _ = sleep(Duration::from_millis(30)) => {
                    println!("job {i} completed normally");
                }
                _ = child.cancelled() => {
                    println!("job {i} observed cancellation and stopped early");
                }
            }
        });
    }

    sleep(Duration::from_millis(5)).await;

    // Shutdown sequence: (1) stop intake, (2) cancel to unblock any job
    // that's cooperatively checking the token, (3) drain with a bound.
    accepting.store(false, Ordering::SeqCst);
    root_token.cancel();

    let drain = async {
        while in_flight.join_next().await.is_some() {}
    };
    match timeout(Duration::from_millis(200), drain).await {
        Ok(_) => println!("all in-flight jobs drained cleanly"),
        Err(_) => println!("shutdown deadline exceeded with jobs still running"),
    }
}
