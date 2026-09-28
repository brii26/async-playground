// Scenario: #[tokio::main] defaults to the multi-thread runtime (work-
// stealing across N OS threads, N = num_cpus by default). The
// current_thread runtime runs everything on ONE OS thread -- tasks are
// still concurrent (interleaved at await points) but never parallel.
// The trap: a CPU-bound async task (one that never awaits during a
// long computation) on current_thread STARVES every other task on the
// entire runtime, because there is no other thread to steal work onto
// -- unlike Go, where the scheduler can always run other goroutines on
// other Ps even if one goroutine never yields (Go also preempts
// long-running goroutines since 1.14; Rust's async has NO preemption at
// all -- a task only yields at explicit .await points, so a tight loop
// with no await blocks the executor thread indefinitely).
use tokio::runtime::Builder;
use tokio::time::{sleep, Duration, Instant};

fn cpu_bound_no_yield(n: u64) -> u64 {
    (0..n).sum() // no .await anywhere -- runs to completion uninterrupted
}

fn main() {
    // current_thread: the CPU-bound task starves the timer task.
    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let start = Instant::now();
        let timer = tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
            println!("timer task fired");
        });
        tokio::spawn(async {
            let _ = cpu_bound_no_yield(2_000_000_000); // blocks the ONLY thread
        });
        timer.await.unwrap();
        println!("current_thread: timer resolved after {:?} (delayed by the CPU-bound task hogging the single thread)", start.elapsed());
    });

    // multi_thread: a second OS thread keeps the timer on schedule.
    let rt = Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    rt.block_on(async {
        let start = Instant::now();
        let timer = tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
            println!("timer task fired");
        });
        tokio::spawn(async {
            let _ = cpu_bound_no_yield(2_000_000_000);
        });
        timer.await.unwrap();
        println!("multi_thread: timer resolved after {:?} (a second worker thread ran it independently)", start.elapsed());
    });

    println!("takeaway: CPU-bound work with no .await starves current_thread entirely and steals a whole worker on multi_thread -- use spawn_blocking for it either way");
}
