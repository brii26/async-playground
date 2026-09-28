# async-playground

Go (goroutines) and Rust (tokio) concurrency scenarios. Each scenario is
independently runnable and self-documenting via inline comments.

## Run everything (sanity check)

```
go build ./...
cargo build --workspace
```

## Run a Go scenario

```
go run ./go/<topic>/<scenario>
```

Examples:

```
go run ./go/mutex/lock_across_await
go run ./go/patterns/circuit_breaker
go run ./go/leak_detection/blocked_send_leak
```

Go scenarios:

| Topic | Scenario |
|---|---|
| mutex | `lock_across_await`, `rwmutex_writer_starvation` |
| waitgroup | `add_after_wait`, `leak_on_early_return` |
| semaphore | `weighted_fairness` |
| channels | `select_fairness_and_backpressure` |
| atomics_and_memory_model | `false_sharing_and_happens_before` |
| condvar | `spurious_wakeup` |
| cancellation | `context_tree_propagation` |
| graceful_shutdown | `drain_in_flight` |
| panic_error_propagation | `goroutine_panic_crashes_process` |
| leak_detection | `blocked_send_leak` |
| go_scheduler | `gomaxprocs_and_lockosthread` |
| blocking_offload | `worker_pool_for_blocking_calls` |
| patterns | `fan_in_fan_out`, `worker_pool_backpressure`, `pipeline`, `rate_limiting`, `circuit_breaker`, `actor_model` |

## Run a Rust scenario

Each scenario is its own crate (no `src/bin/`), run by package name via
the workspace root:

```
cargo run -p <scenario>
```

Examples:

```
cargo run -p permit_raii_and_fairness
cargo run -p select_cancellation_safety
cargo run -p self_referential_and_send_bounds
```

Rust scenarios (package name is the scenario, grouped by topic folder):

| Topic | Package(s) |
|---|---|
| mutex | `tokio_mutex_vs_std_across_await` |
| waitgroup | `joinset_structured_join` |
| semaphore | `permit_raii_and_fairness` |
| channels | `mpsc_oneshot_broadcast_semantics` |
| atomics_and_memory_model | `ordering_correctness` |
| condvar | `notify_vs_cond` |
| cancellation | `select_cancellation_safety` |
| graceful_shutdown | `drain_with_cancellation_token` |
| panic_error_propagation | `joinhandle_panic_boundary` |
| leak_detection | `detached_spawn_leak` |
| tokio_runtime | `multithread_vs_current_thread` |
| blocking_offload | `spawn_blocking_vs_block_in_place` |
| patterns | `fan_in_fan_out`, `worker_pool_backpressure`, `pipeline`, `rate_limiting`, `circuit_breaker`, `actor_model` |
| pin_send_sync | `self_referential_and_send_bounds` |
