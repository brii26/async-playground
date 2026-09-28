// Scenario: holding a mutex across a blocking/slow operation.
//
// The bug here isn't a data race -- it's throughput collapse. Go's
// sync.Mutex has no concept of "critical section too long"; it will
// happily let one goroutine hold the lock while doing network I/O,
// serializing every other goroutine behind it. This is the single most
// common cause of "my service has 64 cores and does the work of one".
package main

import (
	"fmt"
	"sync"
	"time"
)

type cache struct {
	mu   sync.Mutex
	data map[string]string
}

// BAD: the simulated remote fetch happens *while holding* mu. Every
// other goroutine calling Get for a different key still blocks, even
// though they share nothing but the lock.
func (c *cache) getBad(key string) string {
	c.mu.Lock()
	defer c.mu.Unlock()
	if v, ok := c.data[key]; ok {
		return v
	}
	time.Sleep(50 * time.Millisecond) // stand-in for a DB/network call
	v := "fetched:" + key
	c.data[key] = v
	return v
}

// GOOD: shrink the critical section to only the map access. The slow
// path (miss) still needs care -- see singleflight-style dedupe below,
// otherwise concurrent misses on the same key all pay the fetch cost
// (thundering herd), which is a *different* problem than lock hold time.
func (c *cache) getGood(key string) string {
	c.mu.Lock()
	if v, ok := c.data[key]; ok {
		c.mu.Unlock()
		return v
	}
	c.mu.Unlock()

	v := "fetched:" + key
	time.Sleep(50 * time.Millisecond)

	c.mu.Lock()
	c.data[key] = v
	c.mu.Unlock()
	return v
}

func main() {
	c := &cache{data: map[string]string{}}
	start := time.Now()
	var wg sync.WaitGroup
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			c.getBad(fmt.Sprintf("k%d", i%4))
		}(i)
	}
	wg.Wait()
	fmt.Println("bad (serialized on I/O):", time.Since(start))

	c2 := &cache{data: map[string]string{}}
	start = time.Now()
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			c2.getGood(fmt.Sprintf("k%d", i%4))
		}(i)
	}
	wg.Wait()
	fmt.Println("good (lock released during I/O):", time.Since(start))
}
