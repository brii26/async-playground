// Scenario: context cancellation propagates down a tree, but a leaf
// goroutine that ignores ctx.Done() will not be interrupted -- context
// is cooperative, not preemptive. Also: cancelling a parent cancels all
// derived children, but a child's own WithCancel does NOT propagate
// upward, and each derived context needs its cancel func called (or
// chained to a parent) to avoid a "context leak" (the context and its
// goroutine bookkeeping stays alive until GC of the cancel closure,
// vet flags this as `lostcancel`).
package main

import (
	"context"
	"fmt"
	"time"
)

func cooperative(ctx context.Context, id string) {
	for {
		select {
		case <-ctx.Done():
			fmt.Printf("%s: stopping, cause=%v\n", id, context.Cause(ctx))
			return
		case <-time.After(10 * time.Millisecond):
		}
	}
}

func uncooperative(id string) {
	// BUG: this goroutine has no ctx.Done() check at all -- it will run
	// to completion (or forever) regardless of the parent's cancellation.
	// Cancellation cannot preempt running code; it can only be observed
	// at points the code chooses to check.
	time.Sleep(200 * time.Millisecond)
	fmt.Printf("%s: finished on its own schedule, ignored cancellation\n", id)
}

func main() {
	parent, cancel := context.WithCancelCause(context.Background())
	child, childCancel := context.WithTimeout(parent, 500*time.Millisecond)
	defer childCancel() // always defer cancel to release resources, even on success path

	go cooperative(child, "cooperative-leaf")
	go uncooperative("uncooperative-leaf")

	time.Sleep(30 * time.Millisecond)
	cancel(fmt.Errorf("parent decided to stop"))

	time.Sleep(250 * time.Millisecond) // let both goroutines settle for demo output
	fmt.Println("takeaway: cancellation is advisory -- every blocking op in a goroutine must select on ctx.Done()")
}
