// A bounded job queue applies backpressure to producers: once the
// buffer is full, Submit blocks the caller instead of growing memory
// unboundedly. Contrast with an unbounded queue (dangerous under
// sustained overload: OOM) and with dropping (silent data loss unless
// explicitly logged/metered).
package main

import (
	"context"
	"errors"
	"fmt"
	"sync"
)

var ErrQueueFullAndCtxDone = errors.New("submit blocked and context cancelled")

type pool struct {
	jobs chan func()
	wg   sync.WaitGroup
}

func newPool(workers, queueDepth int) *pool {
	p := &pool{jobs: make(chan func(), queueDepth)}
	for i := 0; i < workers; i++ {
		p.wg.Add(1)
		go func() {
			defer p.wg.Done()
			for job := range p.jobs {
				job()
			}
		}()
	}
	return p
}

// Submit applies backpressure: if the bounded queue is full, this call
// blocks the producer until room frees up, OR the context is cancelled
// -- giving the producer a way to abandon submission under overload
// instead of blocking forever.
func (p *pool) submit(ctx context.Context, job func()) error {
	select {
	case p.jobs <- job:
		return nil
	case <-ctx.Done():
		return ErrQueueFullAndCtxDone
	}
}

func (p *pool) close() {
	close(p.jobs)
	p.wg.Wait()
}

func main() {
	p := newPool(2, 3) // 2 workers, queue depth 3 -- deliberately small to force backpressure
	var mu sync.Mutex
	completed := 0

	ctx := context.Background()
	for i := 0; i < 10; i++ {
		i := i
		_ = p.submit(ctx, func() {
			mu.Lock()
			completed++
			mu.Unlock()
			_ = i
		})
	}
	p.close()
	fmt.Println("completed:", completed)
	fmt.Println("takeaway: bounded queue + blocking Submit == backpressure propagates to the producer instead of memory growing unboundedly")
}
