// Fan-out: distribute work across N workers reading from one channel.
// Fan-in: merge N result channels into one, using a WaitGroup to know
// when to close the merged channel -- closing too early drops results
// still in flight from a slower worker.
package main

import (
	"fmt"
	"sync"
)

func fanOut(in <-chan int, workers int, fn func(int) int) []<-chan int {
	outs := make([]<-chan int, workers)
	for i := 0; i < workers; i++ {
		out := make(chan int)
		outs[i] = out
		go func(out chan<- int) {
			defer close(out)
			for v := range in {
				out <- fn(v)
			}
		}(out)
	}
	return outs
}

func fanIn(cs ...<-chan int) <-chan int {
	merged := make(chan int)
	var wg sync.WaitGroup
	wg.Add(len(cs))
	for _, c := range cs {
		go func(c <-chan int) {
			defer wg.Done()
			for v := range c {
				merged <- v
			}
		}(c)
	}
	// Closer goroutine: only closes merged AFTER every source is
	// drained, preventing a send-on-closed-channel panic from a
	// still-active fan-out worker.
	go func() {
		wg.Wait()
		close(merged)
	}()
	return merged
}

func main() {
	in := make(chan int)
	go func() {
		defer close(in)
		for i := 1; i <= 20; i++ {
			in <- i
		}
	}()

	outs := fanOut(in, 4, func(v int) int { return v * v })
	sum := 0
	for v := range fanIn(outs...) {
		sum += v
	}
	fmt.Println("sum of squares:", sum)
}
