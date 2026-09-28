// Scenario: a goroutine leaks forever because it's blocked sending on
// an unbuffered channel that nobody will ever receive from again (e.g.
// the receiver returned early due to a timeout). This is the classic
// "fire and forget goroutine + unbuffered result channel" leak.
package main

import (
	"context"
	"fmt"
	"runtime"
	"time"
)

// leaky: if ctx times out before the worker finishes, main abandons the
// channel, but the worker goroutine is still trying to send on it --
// forever, since nothing will ever receive. The goroutine, its stack,
// and everything it references are leaked for the life of the process.
func leaky(ctx context.Context) (int, error) {
	resultCh := make(chan int) // unbuffered -- send blocks until received
	go func() {
		time.Sleep(200 * time.Millisecond) // slow work
		resultCh <- 42                     // blocks forever if nobody's listening
	}()

	select {
	case v := <-resultCh:
		return v, nil
	case <-ctx.Done():
		return 0, ctx.Err() // we give up, but the goroutine above doesn't know
	}
}

// fixed: buffer the channel by 1 so the send can always complete even
// if nobody ever receives -- the goroutine then exits and is GC'd.
func fixed(ctx context.Context) (int, error) {
	resultCh := make(chan int, 1) // buffered -- send never blocks
	go func() {
		time.Sleep(200 * time.Millisecond)
		resultCh <- 42 // completes immediately into the buffer, goroutine exits
	}()

	select {
	case v := <-resultCh:
		return v, nil
	case <-ctx.Done():
		return 0, ctx.Err()
	}
}

func main() {
	before := runtime.NumGoroutine()

	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Millisecond)
	_, _ = leaky(ctx)
	cancel()
	time.Sleep(10 * time.Millisecond)
	fmt.Println("after leaky() timeout, extra goroutines still blocked:", runtime.NumGoroutine()-before)

	time.Sleep(250 * time.Millisecond) // wait out the leaked goroutine's send so it's fair to compare
	before = runtime.NumGoroutine()

	ctx2, cancel2 := context.WithTimeout(context.Background(), 20*time.Millisecond)
	_, _ = fixed(ctx2)
	cancel2()
	time.Sleep(250 * time.Millisecond)
	fmt.Println("after fixed() timeout, extra goroutines still blocked:", runtime.NumGoroutine()-before)
}
