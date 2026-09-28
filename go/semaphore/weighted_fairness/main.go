// Scenario: weighted semaphore fairness (golang.org/x/sync/semaphore
// pattern, hand-rolled here with a buffered channel to stay stdlib-only).
//
// A channel-of-tokens semaphore is FIFO-ish but NOT strictly fair under
// the Go scheduler: goroutines racing to receive from the same channel
// are woken in roughly FIFO order by the runtime's sudog wait queue,
// but "roughly" matters -- a goroutine that re-acquires quickly in a
// tight loop can win against one that was waiting longer if the
// scheduler happens to run it first after both are unblocked
// simultaneously (e.g. many releases at once). The takeaway for L6
// design: don't rely on channel-based semaphores for fairness
// guarantees under bursty release patterns; if fairness is a hard
// requirement, use an explicit ticket/sequence number.
package main

import (
	"fmt"
	"sync"
)

type weightedSem struct {
	tokens chan struct{}
}

func newWeightedSem(n int) *weightedSem {
	return &weightedSem{tokens: make(chan struct{}, n)}
}

func (s *weightedSem) acquire() { s.tokens <- struct{}{} }
func (s *weightedSem) release() { <-s.tokens }

func main() {
	sem := newWeightedSem(3) // max 3 concurrent holders
	var wg sync.WaitGroup
	var mu sync.Mutex
	order := []int{}

	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			sem.acquire()
			defer sem.release()
			mu.Lock()
			order = append(order, i)
			mu.Unlock()
		}(i)
	}
	wg.Wait()
	fmt.Println("acquisition order (not guaranteed == spawn order):", order)
	fmt.Println("takeaway: semaphore limits concurrency, not fairness -- add a ticket queue if ordering matters")
}
