// Scenario: a panic in a goroutine that is NOT recovered crashes the
// ENTIRE process, not just that goroutine -- unlike an error return,
// panics do not respect goroutine boundaries for propagation (there is
// no propagation; there's just process death unless recovered locally).
// Every goroutine you spawn in a production system needs its own
// recover() at the top if you don't want one bad input to take down
// the whole service.
package main

import (
	"fmt"
	"sync"
)

func safeGo(wg *sync.WaitGroup, name string, fn func()) {
	wg.Add(1)
	go func() {
		defer wg.Done()
		defer func() {
			if r := recover(); r != nil {
				// FIX: recover happens here, at the top of the
				// goroutine's own stack. A recover() in the CALLER
				// (main) would NOT catch this -- recover only works
				// within the same goroutine's deferred call chain.
				fmt.Printf("recovered panic in %s: %v\n", name, r)
			}
		}()
		fn()
	}()
}

func main() {
	var wg sync.WaitGroup

	safeGo(&wg, "worker-1", func() {
		var m map[string]int
		m["x"] = 1 // panics: assignment to entry in nil map
	})

	safeGo(&wg, "worker-2", func() {
		fmt.Println("worker-2 completed normally")
	})

	wg.Wait()
	fmt.Println("main survived both goroutines because each had its own recover()")
	// Without safeGo's recover, the nil-map write in worker-1 would
	// terminate the whole process with a stack trace, taking worker-2's
	// in-flight work down with it regardless of its own correctness.
}
