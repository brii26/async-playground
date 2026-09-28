// Same breaker semantics as the Go version, but the half-open
// "exactly one trial in flight" invariant is easier to get right here:
// std::sync::atomic::AtomicBool::compare_exchange gives a lock-free,
// single-CAS way to claim the trial slot, versus the Go version's
// mutex + bool field (also correct, just heavier for this one flag).
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

struct Breaker {
    open: AtomicBool,
    trial_in_flight: AtomicBool,
    failures: AtomicU32,
    threshold: u32,
    opened_at: std::sync::Mutex<Option<Instant>>,
    cooldown: Duration,
}

impl Breaker {
    fn new(threshold: u32, cooldown: Duration) -> Self {
        Self {
            open: AtomicBool::new(false),
            trial_in_flight: AtomicBool::new(false),
            failures: AtomicU32::new(0),
            threshold,
            opened_at: std::sync::Mutex::new(None),
            cooldown,
        }
    }

    async fn call<F, Fut>(&self, f: F) -> Result<(), &'static str>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), &'static str>>,
    {
        if self.open.load(Ordering::SeqCst) {
            let elapsed = self.opened_at.lock().unwrap().map(|t| t.elapsed());
            if elapsed.map_or(true, |e| e < self.cooldown) {
                return Err("circuit open");
            }
            // Cooldown elapsed: only ONE caller wins the CAS and gets to trial.
            if self
                .trial_in_flight
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return Err("circuit open (trial already in flight)");
            }
        }

        let result = f().await;
        self.trial_in_flight.store(false, Ordering::SeqCst);
        match result {
            Ok(()) => {
                self.open.store(false, Ordering::SeqCst);
                self.failures.store(0, Ordering::SeqCst);
                Ok(())
            }
            Err(e) => {
                let f = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
                if f >= self.threshold || self.open.load(Ordering::SeqCst) {
                    self.open.store(true, Ordering::SeqCst);
                    *self.opened_at.lock().unwrap() = Some(Instant::now());
                }
                Err(e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let breaker = Arc::new(Breaker::new(3, Duration::from_millis(100)));
    for i in 0..5 {
        let r = breaker.call(|| async { Err("downstream failure") }).await;
        println!("call {i}: {r:?}");
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    let r = breaker.call(|| async { Ok(()) }).await;
    println!("trial after cooldown: {r:?}");
}
