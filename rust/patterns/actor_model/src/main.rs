// Actor model in Rust: an enum message type (instead of Go's `any` +
// type switch) gives compile-time exhaustiveness checking on the
// actor's match -- adding a new message variant forces every actor
// handling it to be updated, caught at compile time rather than at
// runtime via a missed type-switch case in Go.
use tokio::sync::{mpsc, oneshot};

enum Msg {
    Add(i64),
    Get(oneshot::Sender<i64>),
}

struct CounterActor {
    inbox: mpsc::Receiver<Msg>,
    state: i64,
}

impl CounterActor {
    async fn run(mut self) {
        while let Some(msg) = self.inbox.recv().await {
            match msg {
                Msg::Add(delta) => self.state += delta, // only this task ever touches state
                Msg::Get(reply) => {
                    let _ = reply.send(self.state);
                }
            }
        }
    }
}

#[derive(Clone)]
struct CounterHandle {
    tx: mpsc::Sender<Msg>,
}

impl CounterHandle {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(CounterActor { inbox: rx, state: 0 }.run());
        Self { tx }
    }

    async fn add(&self, delta: i64) {
        self.tx.send(Msg::Add(delta)).await.unwrap();
    }

    async fn get(&self) -> i64 {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(Msg::Get(reply_tx)).await.unwrap();
        reply_rx.await.unwrap()
    }
}

#[tokio::main]
async fn main() {
    let actor = CounterHandle::new();
    let mut handles = vec![];
    for _ in 0..100 {
        let actor = actor.clone();
        handles.push(tokio::spawn(async move { actor.add(1).await }));
    }
    for h in handles {
        h.await.unwrap();
    }
    println!("final count: {}", actor.get().await);
}
