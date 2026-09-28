// Scenario: false sharing (cache-line contention) between unrelated
// atomic counters, and the Go memory model's happens-before rule for
// atomics vs plain reads.
//
// Two int64 counters that are logically independent but live in the
// same 64-byte cache line will ping-pong that line between CPU cores
// on every write, even though there's no data race -- this is a pure
// performance bug invisible to the race detector. Pad to cache-line
// size to fix it. Second point: sync/atomic operations establish
// happens-before edges (as of the Go memory model spec formalized in
// Go 1.19), but a PLAIN read/write of the same variable elsewhere in
// the program is still a data race even if all the atomic accesses are
// correct -- atomicity is per-operation, not per-variable, unless every
// access to that variable is atomic.
package main

import (
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
)

type unpadded struct {
	a int64
	b int64
}

type padded struct {
	a int64
	_ [56]byte // pad to 64 bytes so b lands on a different cache line
	b int64
}

func bench(name string, incA, incB func()) {
	var wg sync.WaitGroup
	n := runtime.GOMAXPROCS(0)
	start := time.Now()
	const iters = 2_000_000
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			for j := 0; j < iters; j++ {
				if i%2 == 0 {
					incA()
				} else {
					incB()
				}
			}
		}(i)
	}
	wg.Wait()
	fmt.Println(name, time.Since(start))
}

func main() {
	u := &unpadded{}
	bench("unpadded (false sharing)", func() { atomic.AddInt64(&u.a, 1) }, func() { atomic.AddInt64(&u.b, 1) })

	p := &padded{}
	bench("padded (no false sharing)", func() { atomic.AddInt64(&p.a, 1) }, func() { atomic.AddInt64(&p.b, 1) })
}
