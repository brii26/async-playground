// Scenario: Go's WaitGroup has no equivalent to Rust's JoinSet result
// aggregation -- WaitGroup only tells you "everyone finished", not
// "here are the individual outcomes, including which task panicked".
// tokio::task::JoinSet gives structured concurrency: spawn N tasks,
// then join_next() drains results as they complete (not in spawn
// order), and a panicked task surfaces as an Err from join_next()
// instead of silently crashing the process (contrast with Go, where an
// unrecovered goroutine panic DOES crash the process -- Rust's JoinSet
// catches the unwind at the task boundary by design).
use tokio::task::JoinSet;

#[tokio::main]
async fn main() {
    let mut set: JoinSet<i32> = JoinSet::new();
    for i in 0..5 {
        set.spawn(async move {
            if i == 3 {
                panic!("task {i} failed on purpose");
            }
            i * i
        });
    }

    let mut sum = 0;
    let mut panics = 0;
    while let Some(res) = set.join_next().await {
        match res {
            Ok(v) => sum += v,
            Err(e) if e.is_panic() => panics += 1,
            Err(e) => eprintln!("task cancelled: {e}"),
        }
    }
    println!("sum of surviving results: {sum}, panicked tasks: {panics}");
    println!("takeaway: JoinSet isolates panics per-task; the process survives, unlike an unrecovered Go goroutine panic");
}
