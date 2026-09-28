// Scenario: sync.Cond requires re-checking the condition in a loop, not
// an if, because Wait() can return without the condition actually being
// true (spurious wakeup is possible in principle, and more importantly,
// Broadcast() wakes ALL waiters even if only one should proceed --
// every waiter must re-verify state itself).
package main

import (
	"fmt"
	"sync"
)

func main() {
	var mu sync.Mutex
	cond := sync.NewCond(&mu)
	queue := []int{}
	done := false

	var wg sync.WaitGroup
	for i := 0; i < 3; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			mu.Lock()
			// BUG if this were `if len(queue) == 0 { cond.Wait() }`:
			// after Broadcast, multiple goroutines wake, race to grab
			// the item, and one finds the queue empty again.
			for len(queue) == 0 && !done {
				cond.Wait() // atomically unlocks mu, sleeps, re-locks on wake
			}
			if len(queue) > 0 {
				item := queue[0]
				queue = queue[1:]
				fmt.Printf("worker %d got item %d\n", id, item)
			}
			mu.Unlock()
		}(i)
	}

	mu.Lock()
	queue = append(queue, 42)
	mu.Unlock()
	cond.Broadcast() // wakes all 3 waiters; only one will find the item

	mu.Lock()
	done = true
	mu.Unlock()
	cond.Broadcast()

	wg.Wait()
}
