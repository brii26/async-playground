// Scenario: RWMutex writer starvation under sustained reader load.
//
// Go's sync.RWMutex is NOT starvation-free by API guarantee in the way
// some other languages' implementations are, but in practice the Go
// runtime *does* bias against writer starvation since Go 1.x by making
// a pending writer block new readers from acquiring once it's waiting
// (see runtime/rwmutex.go: writer sets a flag that stops further
// RLock() from succeeding until it gets in). The subtlety worth
// internalizing: this means a writer can still wait arbitrarily long
// behind readers *already holding* the lock when it arrives, and
// heavy read contention degrades to something close to a regular
// Mutex in effective throughput once a writer is queued, because every
// new reader now blocks too. Design implication: RWMutex only pays off
// when reads are long relative to lock overhead AND writes are rare
// AND you can tolerate a write-side latency spike when it does happen.
package main

import (
	"fmt"
	"sync"
	"sync/atomic"
	"time"
)

func main() {
	var mu sync.RWMutex
	var reads int64
	stop := make(chan struct{})

	// Saturate with readers holding the lock briefly but continuously.
	var wg sync.WaitGroup
	for i := 0; i < 16; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for {
				select {
				case <-stop:
					return
				default:
				}
				mu.RLock()
				atomic.AddInt64(&reads, 1)
				time.Sleep(time.Microsecond) // simulate read work
				mu.RUnlock()
			}
		}()
	}

	// One writer arrives after readers are already saturating the lock.
	time.Sleep(5 * time.Millisecond)
	writeStart := time.Now()
	mu.Lock()
	writeWait := time.Since(writeStart)
	mu.Unlock()

	close(stop)
	wg.Wait()

	fmt.Printf("writer waited %v behind %d reads to acquire\n", writeWait, atomic.LoadInt64(&reads))
	fmt.Println("takeaway: writer wait time scales with reader saturation, not just queue position")
}
