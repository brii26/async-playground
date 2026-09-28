// Scenario: a panic inside a tokio::spawn'd task does NOT crash the
// process (unlike an unrecovered Go goroutine panic, which kills
// everything) -- tokio catches the unwind at the task boundary and
// surfaces it as an Err(JoinError) from the JoinHandle, where
// JoinError::is_panic() is true. The catch: if you NEVER await the
// JoinHandle (fire-and-forget spawn), the panic is silently swallowed
// -- logged by tokio's default panic hook to stderr, but nothing in
// your program's control flow ever learns about it. This is the
// inverse failure mode of Go: Go fails loud by default (crash) unless
// you add recover(); Rust fails silent by default (task just stops)
// unless you await the handle and check the result.
#[tokio::main]
async fn main() {
    // Awaited: panic surfaces as an error we can handle.
    let handle = tokio::spawn(async {
        panic!("deliberate panic inside a task");
    });
    match handle.await {
        Ok(_) => println!("task completed"),
        Err(e) if e.is_panic() => println!("caught task panic via JoinError: {e}"),
        Err(e) => println!("task was cancelled: {e}"),
    }

    // Fire-and-forget: panic is swallowed by our own code entirely --
    // only tokio's internal default panic hook prints to stderr; our
    // program has no way to react to it programmatically.
    tokio::spawn(async {
        panic!("this panic is never observed by any JoinHandle");
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    println!("main continues, unaware the fire-and-forget task panicked");
    println!("takeaway: always retain and eventually await JoinHandles (or use JoinSet) if a task's failure must be actionable");
}
