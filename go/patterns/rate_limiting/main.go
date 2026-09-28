// time.Ticker gives a steady rate limit (token bucket refilled at fixed
// intervals). Contrast with a naive time.Sleep loop, which drifts under
// load because sleep duration doesn't account for work time -- Ticker's
// channel-based approach self-corrects because it's driven by wall
// clock ticks, not by "sleep, then do work, then sleep again".
package main

import (
	"fmt"
	"time"
)

func main() {
	limiter := time.NewTicker(50 * time.Millisecond) // 20 req/sec
	defer limiter.Stop()

	burst := make(chan struct{}, 3) // allow a burst of 3 immediately
	for i := 0; i < 3; i++ {
		burst <- struct{}{}
	}

	requests := 8
	start := time.Now()
	for i := 0; i < requests; i++ {
		select {
		case <-burst:
			// spend a burst token, no wait
		case <-limiter.C:
			// steady-state: one token per tick
		}
		fmt.Printf("request %d at %v\n", i, time.Since(start).Round(time.Millisecond))
	}
}
