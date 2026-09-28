// Scenario: select{} pseudo-randomizes among ready cases, and unbuffered
// vs buffered channels give fundamentally different backpressure.
//
// select with multiple ready cases picks uniformly at random -- this is
// intentional (prevents starvation of any one channel) but it means you
// cannot use select to express priority between channels without an
// explicit priority pattern (nested select with a high-priority
// non-blocking check first). Second lesson: an unbuffered channel send
// is a synchronization point (producer blocks until consumer receives
// == backpressure is immediate and precise); a buffered channel decouples
// them up to capacity, which hides backpressure until the buffer fills,
// at which point it becomes a cliff instead of a gradient.
package main

import (
	"fmt"
)

func priorityDrain(high, low <-chan int) {
	for {
		// Non-blocking check on the high-priority channel first.
		select {
		case v, ok := <-high:
			if !ok {
				high = nil
			} else {
				fmt.Println("high:", v)
				continue
			}
		default:
		}

		select {
		case v, ok := <-high:
			if !ok {
				high = nil
				continue
			}
			fmt.Println("high:", v)
		case v, ok := <-low:
			if !ok {
				low = nil
				continue
			}
			fmt.Println("low:", v)
		}
		if high == nil && low == nil {
			return
		}
	}
}

func main() {
	high := make(chan int, 2)
	low := make(chan int, 2)
	high <- 1
	low <- 100
	high <- 2
	close(high)
	low <- 200
	close(low)
	priorityDrain(high, low)
	fmt.Println("takeaway: plain select gives no priority; you must build it via non-blocking pre-check")
}
