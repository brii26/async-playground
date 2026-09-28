// Multi-stage pipeline where each stage is a goroutine reading from one
// channel and writing to the next. Key discipline: every stage must
// close its OUTPUT channel when its INPUT channel closes and drains --
// otherwise downstream stages range forever on a channel that will
// never close, leaking the whole pipeline tail.
package main

import "fmt"

func generate(nums ...int) <-chan int {
	out := make(chan int)
	go func() {
		defer close(out)
		for _, n := range nums {
			out <- n
		}
	}()
	return out
}

func square(in <-chan int) <-chan int {
	out := make(chan int)
	go func() {
		defer close(out) // propagate closure downstream
		for n := range in {
			out <- n * n
		}
	}()
	return out
}

func filterEven(in <-chan int) <-chan int {
	out := make(chan int)
	go func() {
		defer close(out)
		for n := range in {
			if n%2 == 0 {
				out <- n
			}
		}
	}()
	return out
}

func main() {
	stage1 := generate(1, 2, 3, 4, 5, 6)
	stage2 := square(stage1)
	stage3 := filterEven(stage2)

	for v := range stage3 {
		fmt.Println(v)
	}
}
