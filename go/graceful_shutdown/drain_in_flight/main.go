// Scenario: graceful shutdown must stop accepting NEW work before
// waiting for IN-FLIGHT work to drain, and the ordering matters. Doing
// it in the wrong order (wait first, close intake second) means new
// work can keep arriving indefinitely and Wait() never returns.
package main

import (
	"context"
	"fmt"
	"sync"
	"time"
)

type server struct {
	wg      sync.WaitGroup
	intake  chan int
	closeMu sync.Mutex
	closed  bool
}

func newServer() *server {
	return &server{intake: make(chan int, 100)}
}

func (s *server) submit(job int) bool {
	s.closeMu.Lock()
	defer s.closeMu.Unlock()
	if s.closed {
		return false // reject new work once shutdown has begun
	}
	s.wg.Add(1)
	go func() {
		defer s.wg.Done()
		time.Sleep(20 * time.Millisecond) // simulate work
	}()
	return true
}

// Shutdown: (1) stop accepting new work, (2) THEN wait for in-flight to
// drain, bounded by ctx so shutdown itself can time out.
func (s *server) shutdown(ctx context.Context) error {
	s.closeMu.Lock()
	s.closed = true
	s.closeMu.Unlock()

	done := make(chan struct{})
	go func() {
		s.wg.Wait()
		close(done)
	}()

	select {
	case <-done:
		return nil
	case <-ctx.Done():
		return fmt.Errorf("shutdown timed out with work still in flight: %w", ctx.Err())
	}
}

func main() {
	s := newServer()
	for i := 0; i < 5; i++ {
		s.submit(i)
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := s.shutdown(ctx); err != nil {
		fmt.Println("shutdown error:", err)
		return
	}
	fmt.Println("all in-flight work drained; new submissions after this point:", s.submit(99))
}
