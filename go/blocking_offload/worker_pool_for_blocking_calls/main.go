// Scenario: a goroutine that makes a blocking syscall (file I/O, cgo
// call without releasing the GIL-equivalent, etc.) does NOT block other
// goroutines from running, because the Go runtime detects the blocking
// syscall and hands the P off to another M (OS thread) -- this is
// exactly why goroutines are cheap for I/O-bound concurrency without
// needing manual thread-pool sizing, UNLIKE a naive thread-per-request
// model in other languages. The place this breaks down: CPU-bound work
// disguised as "just a function call" (e.g. heavy JSON marshaling,
// image processing) does NOT yield the P -- it runs to completion on
// its P, and with GOMAXPROCS=N, only N such goroutines make progress
// simultaneously no matter how many you spawn. That's correct and
// expected; the mistake is spawning 10,000 goroutines for CPU-bound
// work expecting 10,000-way parallelism -- you get GOMAXPROCS-way
// parallelism with scheduling overhead on top.
package main

import (
	"fmt"
	"runtime"
	"sync"
	"time"
)

func cpuBound(n int) int {
	sum := 0
	for i := 0; i < n; i++ {
		sum += i * i
	}
	return sum
}

func main() {
	fmt.Println("GOMAXPROCS:", runtime.GOMAXPROCS(0))

	// Bounded worker pool sized to GOMAXPROCS: matches available
	// parallelism for CPU-bound work instead of over-spawning.
	jobs := make(chan int, 100)
	results := make(chan int, 100)
	var wg sync.WaitGroup

	for w := 0; w < runtime.GOMAXPROCS(0); w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for n := range jobs {
				results <- cpuBound(n)
			}
		}()
	}

	start := time.Now()
	for i := 0; i < 20; i++ {
		jobs <- 5_000_000
	}
	close(jobs)

	go func() { wg.Wait(); close(results) }()
	total := 0
	for r := range results {
		total += r
	}
	fmt.Println("total:", total, "elapsed:", time.Since(start))
	fmt.Println("takeaway: size CPU-bound worker pools to GOMAXPROCS, not to job count -- more goroutines won't add parallelism, only scheduling overhead")
}
