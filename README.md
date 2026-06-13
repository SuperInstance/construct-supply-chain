# construct-supply-chain

**A staged pipeline for GPU construct delivery** — from git discovery through validation, compilation, and deployment, with per-stage queueing, rejection rates, and throughput metrics. Models the software factory that turns source repos into running GPU kernels.

## Why It Matters

Modern GPU fleets process thousands of construct updates per day: model weight hotploads, kernel recompilations, configuration changes. Each must be discovered, validated, compiled, deployed, and verified — and each stage can reject, cache, or retry. Without a formal pipeline model, you get untracked deployments, broken kernels in production, and no visibility into bottlenecks.

This crate implements a discrete-stage queueing model (analogous to a manufacturing supply chain) where each construct flows through `Discovered → Validating → Compiling → Deploying → Live` (with `Rejected` and `Cached` sinks). Every transition is logged, every stage has a queue, and aggregate throughput is measurable.

## How It Works

### Pipeline Model

The supply chain is a directed graph of FIFO queues:

```
                 ┌──────────┐
Discover → │ Discovered │ ──┐
                 └──────────┘    │
                            ▼
                 ┌──────────┐    │     ┌──────────┐
                 │Validating│ ───┼────▶│ Rejected │
                 └──────────┘    │     └──────────┘
                            ▼
                 ┌──────────┐
                 │Compiling │
                 └──────────┘
                            ▼
                 ┌──────────┐
                 │Deploying │
                 └──────────┘
                            ▼
                 ┌──────────┐
                 │   Live   │
                 └──────────┘
```

### Queueing Theory

Each stage is a `VecDeque<Construct>` — a FIFO queue with:

| Operation | Time Complexity |
|-----------|----------------|
| `discover` (push back) | O(1) |
| `advance` (pop front + push to next) | O(1) |
| `process_all` (drain all) | O(n) total, O(1) per transition |
| `queue_depth()` | O(1)* — cached count |

*Queue depth is computed by summing four queue lengths on each call: O(4) = O(1).

### Little's Law Application

For a stable pipeline in steady state, Little's Law gives:

$$L = \lambda \times W$$

Where:
- L = average number of constructs in the system (queue depth)
- λ = arrival rate (constructs/second into `Discovered`)
- W = average time a construct spends in the pipeline

The `throughput()` method computes `total_deployed / total_time_s`, which is the effective λ at the output. If throughput is low and queue depth is high, W is too long — the compile or deploy stage is the bottleneck.

### Validation Gate

The validation stage applies a score function `f: Construct → f64 ∈ [0, 1]`. Constructs with score > 0.5 advance; others are rejected. The rejection rate is:

$$R_{reject} = \frac{N_{rejected}}{N_{discovered}}$$

### Compile Time Model

Compilation time is modeled as proportional to construct size:

$$t_{compile} = \max\left(\frac{\text{size\_bytes}}{100},\ 10\right) \text{μs}$$

This reflects the reality that larger GPU kernels take longer to compile, with a minimum floor.

## Quick Start

```rust
use construct_supply_chain::SupplyChain;

let mut sc = SupplyChain::new();

// Discover constructs from git repos
sc.discover("attention-kernel", "v2", 4096);
sc.discover("reduce-kernel", "v1", 2048);

// Run the full pipeline
let transitions = sc.process_all();
assert_eq!(sc.live_count(), 2);
assert_eq!(sc.rejected_count(), 0);

// Check throughput
println!("Throughput: {:.2} constructs/s", sc.throughput());
println!("Rejection rate: {:.1}%", sc.rejection_rate() * 100.0);
```

## API

### `SupplyChain`
- `new() -> Self` — Initialize empty pipeline
- `discover(&mut self, name, version, size_bytes)` — Add a construct to the discovery queue
- `advance(&mut self) -> Option<String>` — Process one construct through one stage transition
- `process_all(&mut self) -> Vec<String>` — Drain the pipeline completely
- `live_count(&self) -> usize` — Constructs in `Live` state
- `rejected_count(&self) -> usize` — Constructs in `Rejected` state
- `queue_depth(&self) -> usize` — Total constructs still in pipeline
- `throughput(&self) -> f64` — Deployed constructs per second
- `rejection_rate(&self) -> f64` — Fraction of discovered constructs rejected
- `stats(&self) -> &ChainStats` — Aggregate statistics

### `Stage` (enum)
`Discovered` | `Validating` | `Compiling` | `Deploying` | `Live` | `Rejected` | `Cached`

### `ChainStats`
- `total_discovered`, `total_validated`, `total_compiled`, `total_deployed`, `total_rejected: u64`
- `total_time_us: u64` — Cumulative compile time

## Architecture Notes

The supply chain feeds into the broader SuperInstance orchestration stack:

- **construct-provenance** — every stage transition writes a `ProvenanceEntry` to the audit log
- **fleet-coordinator** — `Live` constructs are registered as fleet nodes for task assignment
- **edge-conservation-rs** — verifies the conservation invariant Σ(active) + Σ(cached) = Σ(discovered) holds across pipeline state changes

The invariant γ + η = C maps directly here: γ is the set of `Live` constructs, η is the set of `Cached`/`Rejected` constructs, and C is the set of all `Discovered` constructs.

See the full architecture: [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md)

## References

1. Little, J.D.C. (1961). "A Proof for the Queuing Formula L = λW." *Operations Research, 9(3), 383–387.*
2. Bass, L., Clements, P., & Kazman, R. (2021). *Software Architecture in Practice,* 4th ed. Addison-Wesley. Chapter on deployment pipelines.
3. Humble, J. & Farley, D. (2010). *Continuous Delivery.* Addison-Wesley.
4. Kubernetes — [kubernetes.io](https://kubernetes.io/) — Production container orchestration with staged rollouts
5. Bazel — [bazel.build](https://bazel.build/) — Hermetic build system with provenance tracking

## License

MIT
