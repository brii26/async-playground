// Minimal circuit breaker: closed (calls pass through) -> open (calls
// fail fast after N consecutive failures) -> half-open (after a cooldown,
// allow ONE trial call; success closes, failure re-opens). The critical
// correctness property: half-open must allow exactly one in-flight
// trial, not a burst -- otherwise a struggling downstream gets hit with
// a thundering herd the instant the cooldown expires, which is the
// exact failure mode the breaker exists to prevent.
package main

import (
	"errors"
	"fmt"
	"sync"
	"time"
)

type state int

const (
	closed state = iota
	open
	halfOpen
)

var ErrOpen = errors.New("circuit open")

type breaker struct {
	mu          sync.Mutex
	st          state
	failures    int
	threshold   int
	cooldown    time.Duration
	openedAt    time.Time
	trialInFlight bool
}

func newBreaker(threshold int, cooldown time.Duration) *breaker {
	return &breaker{threshold: threshold, cooldown: cooldown}
}

func (b *breaker) call(fn func() error) error {
	b.mu.Lock()
	switch b.st {
	case open:
		if time.Since(b.openedAt) < b.cooldown {
			b.mu.Unlock()
			return ErrOpen
		}
		if b.trialInFlight {
			b.mu.Unlock()
			return ErrOpen // another goroutine already owns the trial call
		}
		b.st = halfOpen
		b.trialInFlight = true
	case halfOpen:
		if b.trialInFlight {
			b.mu.Unlock()
			return ErrOpen
		}
		b.trialInFlight = true
	}
	b.mu.Unlock()

	err := fn()

	b.mu.Lock()
	defer b.mu.Unlock()
	b.trialInFlight = false
	if err != nil {
		b.failures++
		if b.st == halfOpen || b.failures >= b.threshold {
			b.st = open
			b.openedAt = time.Now()
		}
		return err
	}
	b.st = closed
	b.failures = 0
	return nil
}

func main() {
	b := newBreaker(3, 100*time.Millisecond)
	failing := true

	for i := 0; i < 6; i++ {
		err := b.call(func() error {
			if failing {
				return errors.New("downstream error")
			}
			return nil
		})
		fmt.Printf("call %d: %v\n", i, err)
	}

	time.Sleep(150 * time.Millisecond)
	failing = false
	err := b.call(func() error { return nil })
	fmt.Println("trial call after cooldown:", err)
}
