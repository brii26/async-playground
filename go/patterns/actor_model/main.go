// Actor model in Go: each actor is a goroutine owning its state
// exclusively, communicating only via its inbound channel -- no shared
// memory, no mutex. This sidesteps data races by construction, at the
// cost of message-passing overhead and the need for a reply channel
// per request if the caller needs a result (request/response over
// channels).
package main

import "fmt"

type getMsg struct{ reply chan int }
type addMsg struct{ delta int }

type counterActor struct {
	inbox chan any
	state int
}

func newCounterActor() *counterActor {
	a := &counterActor{inbox: make(chan any)}
	go a.run()
	return a
}

func (a *counterActor) run() {
	for msg := range a.inbox {
		switch m := msg.(type) {
		case addMsg:
			a.state += m.delta // only this goroutine ever touches state
		case getMsg:
			m.reply <- a.state
		}
	}
}

func (a *counterActor) Add(delta int) { a.inbox <- addMsg{delta} }
func (a *counterActor) Get() int {
	reply := make(chan int)
	a.inbox <- getMsg{reply}
	return <-reply
}
func (a *counterActor) Close() { close(a.inbox) }

func main() {
	actor := newCounterActor()
	defer actor.Close()

	for i := 0; i < 100; i++ {
		actor.Add(1) // safe to call from many goroutines concurrently --
		// serialization happens inside the actor's single-goroutine loop,
		// not via a mutex.
	}
	fmt.Println("final count:", actor.Get())
}
