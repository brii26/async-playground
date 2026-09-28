// Multi-stage pipeline via mpsc, same discipline as Go: each stage
// task must let its own Sender drop when its input closes, so the
// NEXT stage's recv() sees None and exits in turn. Since Sender is
// captured by move into the spawned async block, this happens
// automatically at the end of the task's scope -- no explicit
// close() call needed, unlike Go's `defer close(out)`.
use tokio::sync::mpsc;

fn spawn_stage<F>(mut rx: mpsc::Receiver<i32>, f: F) -> mpsc::Receiver<i32>
where
    F: Fn(i32) -> Option<i32> + Send + 'static,
{
    let (tx, out_rx) = mpsc::channel(8);
    tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            if let Some(mapped) = f(v) {
                if tx.send(mapped).await.is_err() {
                    break; // downstream dropped, stop pulling from upstream
                }
            }
        }
        // tx dropped here automatically -> propagates closure downstream
    });
    out_rx
}

#[tokio::main]
async fn main() {
    let (gen_tx, gen_rx) = mpsc::channel(8);
    tokio::spawn(async move {
        for i in 1..=6 {
            gen_tx.send(i).await.unwrap();
        }
    });

    let squared = spawn_stage(gen_rx, |n| Some(n * n));
    let mut evens = spawn_stage(squared, |n| if n % 2 == 0 { Some(n) } else { None });

    while let Some(v) = evens.recv().await {
        println!("{v}");
    }
}
