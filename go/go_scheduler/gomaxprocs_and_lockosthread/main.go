// Scenario: GOMAXPROCS controls the number of OS threads that can
// execute Go code SIMULTANEOUSLY (logical Ps), not the number of OS
// threads that exist (which can exceed GOMAXPROCS when goroutines block
// in syscalls -- the runtime spins up more Ms to keep P count busy).
// runtime.LockOSThread pins the calling goroutine to its current OS
// thread for the goroutine's lifetime -- required for APIs with
// thread-affinity semantics (some cgo/graphics/OS APIs, e.g. certain
// OpenGL contexts or Windows COM). Without it, the Go scheduler is free
// to migrate a goroutine to a different OS thread between any two
// Go statements (at any preemption/scheduling point), which is invisible
// to normal code but fatal to thread-affine APIs.
package main

import (
	"fmt"
	"runtime"
	"sync"
)

func main() {
	fmt.Println("GOMAXPROCS:", runtime.GOMAXPROCS(0))
	fmt.Println("NumCPU:", runtime.NumCPU())

	var wg sync.WaitGroup
	threadIDs := make([]int, 4)
	for i := 0; i < 4; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			runtime.LockOSThread()
			defer runtime.UnlockOSThread()
			// From here until UnlockOSThread, this goroutine is
			// guaranteed to stay on the same OS thread -- required if
			// this were, e.g., a cgo call that stashed thread-local
			// state the next call depends on.
			_ = i
		}(i)
	}
	wg.Wait()
	_ = threadIDs
	fmt.Println("takeaway: GOMAXPROCS bounds parallel Go execution; LockOSThread is the escape hatch for thread-affine native calls, at the cost of removing that goroutine from the scheduler's normal load-balancing pool")
}
