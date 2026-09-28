// Scenario: this is the one topic with no Go analogue at all -- Go's
// goroutine stacks are moved freely by the runtime (stack growth
// copies the whole stack and fixes up pointers), so self-referential
// data structures simply aren't a concern. Rust async functions
// compile to a state machine struct that CAN be self-referential
// (a local variable borrowed by a `.await`ed sub-future, both living
// in the same generated struct) -- moving that struct after
// initialization would invalidate the internal reference. Pin<Box<T>>
// (or Pin<&mut T>) is the compiler-enforced promise "this value will
// never move again in memory," which is what makes holding such
// borrows across await points sound. You almost never construct Pin
// by hand in application code -- async fn/await does it for you -- but
// understanding WHY it exists explains: (1) why futures aren't Unpin
// by default, (2) why manual Future impls need unsafe or pin-project,
// (3) why "just move this future into a Box" (Box::pin) is the
// standard escape hatch for trait objects (dyn Future).
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// A minimal hand-written self-referential-shaped future to show WHY
// Pin matters: `borrowed` conceptually points into `owned`. Real
// compiler-generated async state machines have exactly this shape
// when a local variable is borrowed across an .await.
struct SelfRef {
    #[allow(dead_code)] // kept to show the conceptual shape of the self-reference
    owned: String,
    borrowed_len: usize, // stand-in for "a pointer into owned" -- using
                          // a real raw pointer here would need unsafe;
                          // this field demonstrates the SHAPE of the
                          // problem without requiring unsafe code.
}

impl Future for SelfRef {
    type Output = usize;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<usize> {
        // Pin<&mut Self> guarantees `self` (and therefore `owned`) has
        // not moved since it was first pinned -- if it could move, any
        // real pointer stored in `borrowed_len`'s place would dangle.
        Poll::Ready(self.borrowed_len)
    }
}

// Send bound deep dive: a future is Send only if EVERY value held
// across its await points is Send. Rc<T> (non-atomic refcount) breaks
// this silently -- the error only appears at the tokio::spawn call
// site, far from the Rc::clone() that caused it, which is the classic
// confusing Rust-async-for-Go-engineers compile error.
fn assert_send<T: Send>(_: T) {}

async fn uses_rc_before_await() {
    use std::rc::Rc;
    // Scoped in a block so `val` (NOT Send) is fully dropped, including
    // in the generated state machine's capture set, before the await
    // below. Simply calling drop(val) is not enough by itself in some
    // rustc versions -- the async-fn lowering can still capture the
    // binding's storage across the yield point unless its lexical
    // scope has actually ended. Scoping is the reliable fix.
    let doubled = {
        let val = Rc::new(5);
        *val * 2
    };
    tokio::task::yield_now().await;
    let _ = doubled; // `doubled` is a plain i32 (Send), fine to hold across await
}

#[tokio::main]
async fn main() {
    let owned = String::from("hello");
    let f = SelfRef { borrowed_len: owned.len(), owned };
    let pinned = Box::pin(f); // Box::pin: heap-allocate + pin in one step,
    // the standard way to get a movable HANDLE (the Box) to an
    // immovable VALUE (the pinned future), which is exactly what `dyn
    // Future` trait objects need since they're accessed only through
    // a pointer.
    let result = pinned.await;
    println!("self-referential-shaped future resolved: {result}");

    let fut = uses_rc_before_await();
    assert_send(fut); // compiles: Rc dropped before the await point
    tokio::spawn(uses_rc_before_await()).await.unwrap();

    println!("takeaway: Pin exists because async state machines can be self-referential; Send bugs come from what's ALIVE across an .await, not what's merely used in the function");
}
