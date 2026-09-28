// Scenario: calling wg.Add() concurrently with wg.Wait() racing to zero.
//
// The WaitGroup counter has no "epoch" -- if Wait() observes the
// counter hit zero and returns while another goroutine is about to
// call Add(1) for a *new* batch of work, that Add can panic
// ("sync: WaitGroup misuse: Add called concurrently with Wait") or,
// worse, silently succeed and race with a Wait() that already
// returned, letting the caller proceed as if all work were done when
// it wasn't. Rule: all Add() calls for a wave must happen-before the
// first Wait() for that wave, from the same goroutine that starts the
// wave. Never Add() from inside the spawned goroutine.
package main

import (
	"fmt"
	"sync"
)

func processWave(wg *sync.WaitGroup, n int, results *[]int, mu *sync.Mutex) {
	// CORRECT: Add() happens in the spawning goroutine, before go func(),
	// establishing happens-before with any concurrent Wait().
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			mu.Lock()
			*results = append(*results, i)
			mu.Unlock()
		}(i)
	}
}

func main() {
	var wg sync.WaitGroup
	var mu sync.Mutex
	results := []int{}

	processWave(&wg, 5, &results, &mu)
	wg.Wait()
	fmt.Println("wave 1 done, results:", len(results))

	// Second wave reuses the same WaitGroup -- safe only because the
	// prior Wait() fully returned (counter at 0, no pending Add race)
	// before this wave's Add() calls begin.
	processWave(&wg, 5, &results, &mu)
	wg.Wait()
	fmt.Println("wave 2 done, results:", len(results))
}
