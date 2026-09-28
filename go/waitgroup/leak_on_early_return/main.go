// Scenario: goroutine leak because the parent returns before Wait(),
// via a code path added later (error return) that forgot the defer.
//
// This is the #1 real-world WaitGroup bug: it's not a deadlock (which
// crashes loudly with "all goroutines are asleep"), it's a *leak* --
// the process keeps running, the goroutines keep existing, and nothing
// ever tells you. Detecting it requires runtime.NumGoroutine() trending
// up over time, pprof goroutine dumps, or a leak-detector test harness
// (e.g. go.uber.org/goleak in real projects).
package main

import (
	"fmt"
	"runtime"
	"sync"
	"time"
)

func leaky(early bool) {
	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		time.Sleep(200 * time.Millisecond)
	}()

	if early {
		return // BUG: wg.Wait() below never runs; caller doesn't block,
		// but the goroutine is still alive and referenced until it
		// finishes on its own -- if it were an infinite loop, it'd
		// leak forever.
	}
	wg.Wait()
}

func fixed(early bool) {
	var wg sync.WaitGroup
	wg.Add(1)
	defer wg.Wait() // FIX: defer ties completion to function exit
	// regardless of which return path is taken.
	go func() {
		defer wg.Done()
		time.Sleep(200 * time.Millisecond)
	}()

	if early {
		return
	}
}

func main() {
	before := runtime.NumGoroutine()
	leaky(true)
	time.Sleep(10 * time.Millisecond) // goroutine hasn't finished yet
	fmt.Println("after leaky(): goroutines in flight:", runtime.NumGoroutine()-before)

	fixed(true) // blocks until goroutine truly finishes, no leak window
	fmt.Println("after fixed(): goroutines in flight:", runtime.NumGoroutine()-before)
}
