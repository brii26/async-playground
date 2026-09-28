// tokio::time::interval is the Rust analogue of Go's time.Ticker: it
// self-corrects against wall-clock drift rather than accumulating
// sleep-then-work error. Missed ticks (if the consumer falls behind)
// default to MissedTickBehavior::Burst (fires all missed ticks back to
// back) -- usually you want Delay or Skip instead for a true rate
// limiter, otherwise a stalled consumer causes a burst that defeats
// the limiter's purpose the moment it catches up.
use tokio::time::{interval, Duration, Instant, MissedTickBehavior};

#[tokio::main]
async fn main() {
    let mut ticker = interval(Duration::from_millis(50));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay); // avoid burst catch-up

    let start = Instant::now();
    for i in 0..5 {
        ticker.tick().await;
        println!("request {i} at {:?}", start.elapsed());
    }
}
