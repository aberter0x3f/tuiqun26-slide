# NOIP IOI-style Hull capacity model

## Scope

This model estimates the continuous CPU capacity needed to provide real-time
feedback if 7,546 NOIP contestants compete for five hours on four tasks. It
reports capacities for 120, 300, and 600 second P95 submission-sojourn targets.
Each capacity is also expressed as a fraction of a 128-core Kunpeng 920 node.

The model contains three sensitivity scenarios:

| Scenario | Expected submissions per contestant | Physical testcases by task |
|---|---:|---:|
| Low | 7.00 | `[16,20,20,16]` |
| Central | 10.81 | `[20,25,25,20]` |
| High | 15.29 | `[28,36,44,48]` |

The scenarios jointly vary submission behavior, testcase fan-out, and runtime
mixture. They are engineering sensitivities rather than statistical confidence
bounds.

## Population and submissions

The fixed population strata are:

| Stratum | Contestants |
|---|---:|
| U | 2,472 |
| B | 1,768 |
| M | 1,761 |
| S | 1,196 |
| E | 349 |
| Total | 7,546 |

For each contestant-task pair, the simulator samples whether the task is
attempted. Conditional submission counts follow a gamma-Poisson mixture with a
shared contestant engagement effect and a limit of 50 submissions per task.
The layer-task probabilities and conditional means are encoded directly in
`src/main.rs`.

Submission times use ten 30-minute probability buckets over the five-hour
contest:

```text
3.55%, 7.35%, 8.31%, 8.57%, 9.41%,
9.83%, 10.39%, 11.24%, 11.58%, 19.78%
```

The final bucket creates the principal short-SLA burst. Contestant-level time
shifts and within-bucket beta draws avoid synchronized arrivals.

## Runtime demand

The service calibration is based on the measured Hull runtime-analysis phase:

```text
47,590,283,381,316 tick / 85 s / 128 cores
```

This gives an effective throughput of
`4,374,106,928.430 tick/(core*s)`. The nominal testcase limit of `2e10 tick`
therefore corresponds to `4.572363 core-s`. This is an end-to-end effective
throughput for the measured workload, including runtime-analysis scheduling and
processing within that phase.

Each submission receives an ability- and task-dependent runtime state:
fast-fail, normal, near-limit, or limit-exceeding. A shared lognormal factor
correlates the testcase runtimes of one submission. The resulting aggregate
demand is the testcase count multiplied by sampled tick demand and converted to
core-seconds using the effective throughput.

## Subtask skip

The model evaluates both full execution and a first-zero approximation. Public
constraint-group counts are used as subtask counts `[13,11,10,10]`, with shared
testcase fractions `[8/20,11/25,7/25,10/20]`.

For a task shape with average `m` testcases per subtask and testcase pass
probability `p`, the expected number evaluated before the first failure is:

```text
E[X] = (1 - p^m) / (1 - p)
```

If `h` is the shared-testcase fraction, the sequential active-union fraction is:

```text
f_seq = h + (1 - h) E[X] / m
```

The simulator assumes that half of the theoretically skippable exclusive work
has already been prefetched when cancellation takes effect:

```text
f_exec = 1 - 0.5 (1 - f_seq)
```

Shared testcases are charged once per submission. The sampled skip workload is
bounded between the shared fraction and full execution.

## Queue and completion approximation

Let submission `n` arrive at `A_n`, require `W_n` core-seconds, and have longest
testcase span `H_n`. For continuous capacity `c`, the aggregate FCFS workload
position is:

```text
B_n = max(A_n, B_(n-1)) + W_n / c
```

Submission sojourn is approximated by:

```text
V_n = max(A_n, B_(n-1)) - A_n + max(W_n / c, H_n)
```

The first term is waiting behind previous aggregate work. The second permits a
submission's testcases to run in parallel while respecting both total service
demand and the longest testcase span. This is a fluid capacity model; it does
not model discrete-core fragmentation or scheduler fairness.

## Monte Carlo capacity statistic

For each scenario the simulator generates 240 complete contests with seed
`20260713`. For each contest, SLA, and skip mode, binary search finds the
continuous core count whose submission-sojourn P95 meets the target. The
reported capacity is the P95 of these 240 required-core values. A 1,000-sample
bootstrap estimates Monte Carlo uncertainty for that statistic.

Replications are distributed across
`std::thread::available_parallelism()` workers. Each replication has a fixed
seed derived from its index, and results are sorted by index before reduction,
so worker scheduling does not affect the CSV.

## Reproduction

From `noip-ioi-capacity/`:

```bash
cargo test --release
cargo build --release
./target/release/noip-ioi-capacity results.csv
./target/release/noip-ioi-capacity /tmp/noip-ioi-capacity-rerun.csv
cmp results.csv /tmp/noip-ioi-capacity-rerun.csv
```

## Interpretation limits

- The submission behavior and runtime mixture are transparent priors because a
  national real-time NOIP trace is unavailable.
- The arrival profile is transferred from recent IOI-style behavior and should
  be replaced when national traces become available.
- The skip calculation approximates active testcase unions without private
  testcase-to-subtask membership or cancellation traces.
- Parallel efficiency is calibrated at one 128-core operating point.
- Capacity is continuous and excludes node rounding, high availability,
  network, storage, and frontend latency.
