use std::cmp::Ordering;
use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::thread;

const SEED: u64 = 20_260_713;
const REPS: usize = 240;
const CONTEST_S: f64 = 18_000.0;
const TL_TICKS: f64 = 20_000_000_000.0;
const MU1: f64 = 47_590_283_381_316.0 / 85.0 / 128.0;
const SLAS: [f64; 3] = [120.0, 300.0, 600.0];
const TIME_P: [f64; 10] = [
  0.0355, 0.0735, 0.0831, 0.0857, 0.0941, 0.0983, 0.1039, 0.1124, 0.1158, 0.1978,
];
const CSV_HEADER: &str = "scenario,SLA_seconds,cores,node_fraction,cores_bootstrap_ci_low,cores_bootstrap_ci_high,cores_replication_p025,cores_replication_p975,submissions,total_ticks,achieved_P95_seconds,replication_P95_sojourn_median_seconds,mean_sojourn_seconds,peak_backlog_submissions,peak_backlog_core_seconds,post_contest_drain_seconds,workload_lower_bound_cores,final30_burst_lower_bound_cores,skip_mode,skip_saving_fraction,assumptions,replications,seed";

#[derive(Clone, Copy)]
struct Cell {
  p: f64,
  c: f64,
}

const fn x(p: f64, c: f64) -> Cell {
  Cell { p, c }
}

const LOW: [[Cell; 4]; 5] = [
  [x(0.92, 1.5), x(0.55, 1.2), x(0.20, 1.1), x(0.15, 1.1)],
  [x(0.98, 2.2), x(0.82, 1.8), x(0.50, 1.5), x(0.42, 1.5)],
  [x(0.99, 3.0), x(0.91, 2.5), x(0.68, 1.9), x(0.60, 2.0)],
  [x(0.995, 4.5), x(0.97, 4.0), x(0.86, 3.2), x(0.82, 3.4)],
  [x(1.0, 6.5), x(0.995, 6.0), x(0.95, 5.0), x(0.93, 5.2)],
];
const CENTRAL: [[Cell; 4]; 5] = [
  [x(0.96, 2.2), x(0.78, 1.8), x(0.42, 1.5), x(0.35, 1.5)],
  [x(0.99, 3.2), x(0.93, 2.8), x(0.72, 2.2), x(0.65, 2.2)],
  [x(0.995, 4.3), x(0.97, 3.8), x(0.84, 3.0), x(0.79, 3.1)],
  [x(0.998, 5.7), x(0.99, 5.1), x(0.94, 4.1), x(0.91, 4.3)],
  [x(1.0, 7.8), x(0.998, 7.2), x(0.98, 6.3), x(0.97, 6.5)],
];
const HIGH: [[Cell; 4]; 5] = [
  [x(0.99, 3.0), x(0.90, 2.5), x(0.62, 2.0), x(0.55, 2.0)],
  [x(1.0, 4.2), x(0.98, 3.8), x(0.85, 3.0), x(0.80, 3.1)],
  [x(1.0, 5.5), x(0.995, 5.0), x(0.93, 4.0), x(0.90, 4.2)],
  [x(1.0, 7.0), x(1.0, 6.5), x(0.98, 5.5), x(0.97, 5.8)],
  [x(1.0, 10.0), x(1.0, 9.5), x(0.995, 8.5), x(0.99, 8.8)],
];

const POPULATIONS: [usize; 5] = [2472, 1768, 1761, 1196, 349];
const LOW_CASES: [usize; 4] = [16, 20, 20, 16];
const CENTRAL_CASES: [usize; 4] = [20, 25, 25, 20];
const HIGH_CASES: [usize; 4] = [28, 36, 44, 48];
const NOIP_SUBTASKS: [usize; 4] = [13, 11, 10, 10];
const NOIP_SHARED_FRACTIONS: [f64; 4] = [8.0 / 20.0, 11.0 / 25.0, 7.0 / 25.0, 10.0 / 20.0];

#[derive(Clone, Copy)]
struct Scenario {
  name: &'static str,
  n: usize,
  cells: &'static [[Cell; 4]; 5],
  dispersion: f64,
  cases: [usize; 4],
}

const SCENARIOS: [Scenario; 3] = [
  Scenario {
    name: "low_noip_N7546",
    n: 7546,
    cells: &LOW,
    dispersion: 2.8,
    cases: LOW_CASES,
  },
  Scenario {
    name: "central_noip_N7546",
    n: 7546,
    cells: &CENTRAL,
    dispersion: 2.2,
    cases: CENTRAL_CASES,
  },
  Scenario {
    name: "high_noip_N7546",
    n: 7546,
    cells: &HIGH,
    dispersion: 1.8,
    cases: HIGH_CASES,
  },
];

#[derive(Clone, Copy)]
struct TaskShape {
  cases: usize,
  subtasks: usize,
  shared_fraction: f64,
}

struct Rng {
  state: u64,
  spare: Option<f64>,
}

impl Rng {
  fn new(seed: u64) -> Self {
    Self {
      state: seed,
      spare: None,
    }
  }
  fn u64(&mut self) -> u64 {
    self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = self.state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
  }
  fn uniform(&mut self) -> f64 {
    ((self.u64() >> 11) as f64 + 0.5) / ((1u64 << 53) as f64)
  }
  fn normal(&mut self) -> f64 {
    if let Some(x) = self.spare.take() {
      return x;
    }
    let r = (-2.0 * self.uniform().ln()).sqrt();
    let t = std::f64::consts::TAU * self.uniform();
    self.spare = Some(r * t.sin());
    r * t.cos()
  }
  fn gamma(&mut self, shape: f64) -> f64 {
    if shape < 1.0 {
      return self.gamma(shape + 1.0) * self.uniform().powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = (1.0 / (9.0 * d)).sqrt();
    loop {
      let x = self.normal();
      let v = (1.0 + c * x).powi(3);
      if v > 0.0 {
        let u = self.uniform();
        if u < 1.0 - 0.0331 * x.powi(4) || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
          return d * v;
        }
      }
    }
  }
  fn poisson(&mut self, lambda: f64) -> usize {
    if lambda < 30.0 {
      let limit = (-lambda).exp();
      let mut product = 1.0;
      let mut k = 0;
      loop {
        k += 1;
        product *= self.uniform();
        if product <= limit {
          return k - 1;
        }
      }
    }
    ((lambda + lambda.sqrt() * self.normal()).round().max(0.0)) as usize
  }
  fn beta(&mut self, a: f64, b: f64) -> f64 {
    let x = self.gamma(a);
    x / (x + self.gamma(b))
  }
}

#[derive(Clone, Copy)]
struct Job {
  arrival: f64,
  skip_core_s: f64,
  full_core_s: f64,
  precedence_s: f64,
}

struct Trace {
  jobs: Vec<Job>,
  cap50_pairs: usize,
  attempted_pairs: usize,
}

fn sigmoid(x: f64) -> f64 {
  1.0 / (1.0 + (-x).exp())
}

fn quantile(mut xs: Vec<f64>, p: f64) -> f64 {
  let z = (xs.len() - 1) as f64 * p;
  let lo = z.floor() as usize;
  let hi = z.ceil() as usize;
  xs.select_nth_unstable_by(lo, |a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
  let low = xs[lo];
  if hi == lo {
    return low;
  }
  let high = xs[lo + 1..].iter().copied().fold(f64::INFINITY, f64::min);
  low + (high - low) * (z - lo as f64)
}

fn choose_bin(rng: &mut Rng) -> usize {
  let u = rng.uniform();
  let mut sum = 0.0;
  for (i, p) in TIME_P.iter().enumerate() {
    sum += p;
    if u <= sum {
      return i;
    }
  }
  9
}

fn expected_active_union_fraction(shape: TaskShape, pass_p: f64) -> f64 {
  let exclusive = 1.0 - shape.shared_fraction;
  let m = (shape.cases as f64 / shape.subtasks as f64).max(1.0);
  let evaluated = if (1.0 - pass_p).abs() < 1e-9 {
    m
  } else {
    (1.0 - pass_p.powf(m)) / (1.0 - pass_p)
  };
  let sequential = shape.shared_fraction + exclusive * (evaluated / m);
  1.0 - 0.5 * (1.0 - sequential)
}

fn tick_derived_precedence_s(runtime_ratio: f64) -> f64 {
  TL_TICKS * runtime_ratio / MU1
}

fn make_trace(scenario: Scenario, rep: usize) -> Trace {
  let scenario_tag = match scenario.name {
    "low_noip_N7546" => 1,
    "central_noip_N7546" => 2,
    "high_noip_N7546" => 3,
    _ => unreachable!(),
  };
  let mut rng = Rng::new(SEED ^ (rep as u64).wrapping_mul(0xd1342543de82ef95) ^ scenario_tag);
  let expected_submissions: f64 = (0..5)
    .map(|z| {
      POPULATIONS[z] as f64
        * (0..4)
          .map(|t| scenario.cells[z][t].p * scenario.cells[z][t].c)
          .sum::<f64>()
    })
    .sum();
  let mut jobs = Vec::with_capacity(expected_submissions as usize);
  let mut cap50 = 0;
  let mut attempted_pairs = 0;
  for person in 0..scenario.n {
    let mut cumulative = 0;
    let mut stratum = 4;
    for (z, population) in POPULATIONS.iter().enumerate() {
      cumulative += population;
      if person < cumulative {
        stratum = z;
        break;
      }
    }
    let ability = [-1.35, -0.65, 0.0, 0.75, 1.45][stratum] + 0.30 * rng.normal();
    let engagement = rng.normal();
    let time_shift = 120.0 * rng.normal();
    for task in 0..4 {
      let cell = scenario.cells[stratum][task];
      if rng.uniform() >= cell.p {
        continue;
      }
      attempted_pairs += 1;
      let multiplier = (0.22 * engagement - 0.5 * 0.22_f64.powi(2) + 0.08 * rng.normal()
        - 0.5 * 0.08_f64.powi(2))
      .exp();
      let extra_mean = (cell.c - 1.0) * multiplier;
      let lambda = rng.gamma(scenario.dispersion) * extra_mean / scenario.dispersion;
      let count = (1 + rng.poisson(lambda)).min(50);
      if count == 50 {
        cap50 += 1;
      }
      for k in 0..count {
        let arrival = ((choose_bin(&mut rng) as f64 + rng.beta(1.2, 1.2)) * 1800.0
          + time_shift
          + 25.0 * k as f64)
          .clamp(0.0, CONTEST_S);
        let quality = ability - [-0.70, -0.05, 0.55, 0.80][task]
          + 0.16 * ((k + 1) as f64).ln()
          + 0.60 * rng.normal();
        let good = sigmoid(quality);
        let mut probs = [
          0.14 + 0.58 * (1.0 - good),
          0.42,
          0.05 + 0.16 * good,
          0.04 + 0.06 * (1.0 - good),
        ];
        let sum: f64 = probs.iter().sum();
        for probability in &mut probs {
          *probability /= sum;
        }
        let u = rng.uniform();
        let mut acc = 0.0;
        let mut state = 3;
        for (i, probability) in probs.iter().enumerate() {
          acc += probability;
          if u <= acc {
            state = i;
            break;
          }
        }
        let runtime_ratio = match state {
          0 => (rng.gamma(2.0) * 0.025_f64).min(0.18),
          1 => rng.beta((0.34 + 0.18 * good) * 9.0, (0.66 - 0.18 * good) * 9.0),
          2 => 0.72 + 0.28 * rng.beta(5.0, 2.0),
          _ => 1.0 + (rng.gamma(2.0) * 0.015).min(0.15),
        }
        .clamp(0.003, 1.15)
          * (0.22 * rng.normal() - 0.5 * 0.22 * 0.22).exp();
        let shape = TaskShape {
          cases: scenario.cases[task],
          subtasks: NOIP_SUBTASKS[task],
          shared_fraction: NOIP_SHARED_FRACTIONS[task],
        };
        let full_ticks = shape.cases as f64 * TL_TICKS * runtime_ratio;
        let pass_p = (0.10 + 0.88 * good).clamp(0.10, 0.98);
        let expected = expected_active_union_fraction(shape, pass_p);
        let noisy = (expected + 0.035 * rng.normal()).clamp(shape.shared_fraction, 1.0);
        let full_core_s = full_ticks / MU1;
        jobs.push(Job {
          arrival,
          skip_core_s: full_core_s * noisy,
          full_core_s,
          precedence_s: tick_derived_precedence_s(runtime_ratio),
        });
      }
    }
  }
  jobs.sort_unstable_by(|a, b| a.arrival.partial_cmp(&b.arrival).unwrap_or(Ordering::Equal));
  Trace {
    jobs,
    cap50_pairs: cap50,
    attempted_pairs,
  }
}

#[derive(Default, Clone)]
struct Eval {
  p95: f64,
  mean: f64,
  peak_backlog: usize,
  peak_core_s: f64,
  drain_s: f64,
}

fn evaluate(trace: &Trace, cores: f64, skip: bool) -> Eval {
  let mut fluid_finish = Vec::with_capacity(trace.jobs.len());
  let mut sojourn = Vec::with_capacity(trace.jobs.len());
  let mut previous = 0.0_f64;
  let mut sum = 0.0;
  let mut queue: VecDeque<f64> = VecDeque::new();
  let mut peak = 0;
  let mut workload_finish = 0.0_f64;
  let mut peak_work = 0.0_f64;
  for job in &trace.jobs {
    let demand = if skip {
      job.skip_core_s
    } else {
      job.full_core_s
    };
    while queue.front().is_some_and(|finish| *finish <= job.arrival) {
      queue.pop_front();
    }
    let batch_start = previous.max(job.arrival);
    let finish = batch_start + demand / cores;
    previous = finish;
    fluid_finish.push(finish);
    queue.push_back(finish);
    peak = peak.max(queue.len());
    let job_sojourn = (batch_start - job.arrival) + (demand / cores).max(job.precedence_s);
    sojourn.push(job_sojourn);
    sum += job_sojourn;
    workload_finish = workload_finish.max(job.arrival) + demand / cores;
    peak_work = peak_work.max((workload_finish - job.arrival) * cores);
  }
  Eval {
    p95: quantile(sojourn, 0.95),
    mean: sum / trace.jobs.len() as f64,
    peak_backlog: peak,
    peak_core_s: peak_work,
    drain_s: (fluid_finish.last().copied().unwrap_or(0.0) - CONTEST_S).max(0.0),
  }
}

fn required_cores(trace: &Trace, sla: f64, skip: bool) -> f64 {
  let mut lo = 0.05;
  let mut hi = 128.0;
  while evaluate(trace, hi, skip).p95 > sla {
    hi *= 2.0;
  }
  for _ in 0..30 {
    let mid = (lo + hi) / 2.0;
    if evaluate(trace, mid, skip).p95 <= sla {
      hi = mid;
    } else {
      lo = mid;
    }
  }
  hi
}

struct RepResult {
  rep: usize,
  trace: Trace,
  required: [[f64; 3]; 2],
}

fn main() -> std::io::Result<()> {
  let args: Vec<String> = env::args().collect();
  let out = args.get(1).expect("usage: noip-ioi-capacity OUTPUT.csv");
  let workers = thread::available_parallelism().map_or(1, usize::from);
  let mut writer = BufWriter::new(File::create(out)?);
  writeln!(writer, "{CSV_HEADER}")?;
  for scenario in SCENARIOS {
    let next = Arc::new(Mutex::new(0usize));
    let results = Arc::new(Mutex::new(Vec::<RepResult>::with_capacity(REPS)));
    let mut handles = Vec::new();
    for _ in 0..workers {
      let next = Arc::clone(&next);
      let results = Arc::clone(&results);
      handles.push(thread::spawn(move || loop {
        let rep = {
          let mut next_rep = next.lock().unwrap();
          if *next_rep >= REPS {
            break;
          }
          let rep = *next_rep;
          *next_rep += 1;
          rep
        };
        let trace = make_trace(scenario, rep);
        let mut required = [[0.0; 3]; 2];
        for (mode, mode_required) in required.iter_mut().enumerate() {
          for (i, sla) in SLAS.iter().enumerate() {
            mode_required[i] = required_cores(&trace, *sla, mode == 0);
          }
        }
        results.lock().unwrap().push(RepResult {
          rep,
          trace,
          required,
        });
      }));
    }
    for handle in handles {
      handle.join().unwrap();
    }
    let mut replications = match Arc::try_unwrap(results) {
      Ok(results) => results.into_inner().unwrap(),
      Err(_) => panic!("worker result Arc still shared"),
    };
    replications.sort_unstable_by_key(|result| result.rep);
    for mode in 0..2 {
      let skip = mode == 0;
      let total_full: f64 = replications
        .iter()
        .flat_map(|r| &r.trace.jobs)
        .map(|j| j.full_core_s)
        .sum();
      let total_used: f64 = replications
        .iter()
        .flat_map(|r| &r.trace.jobs)
        .map(|j| if skip { j.skip_core_s } else { j.full_core_s })
        .sum();
      for (sla_index, sla) in SLAS.iter().enumerate() {
        let required: Vec<f64> = replications
          .iter()
          .map(|r| r.required[mode][sla_index])
          .collect();
        let cores = quantile(required.clone(), 0.95);
        let rep_lo = quantile(required.clone(), 0.025);
        let rep_hi = quantile(required.clone(), 0.975);
        let mut bootstrap_rng = Rng::new(
          SEED
            ^ sla_index as u64
            ^ (mode as u64) << 12
            ^ scenario.name.bytes().map(u64::from).sum::<u64>(),
        );
        let mut bootstrap = Vec::with_capacity(1000);
        for _ in 0..1000 {
          let sample = (0..REPS)
            .map(|_| required[(bootstrap_rng.u64() % REPS as u64) as usize])
            .collect();
          bootstrap.push(quantile(sample, 0.95));
        }
        let ci_lo = quantile(bootstrap.clone(), 0.025);
        let ci_hi = quantile(bootstrap, 0.975);
        let evaluations: Vec<Eval> = replications
          .iter()
          .map(|r| evaluate(&r.trace, cores, skip))
          .collect();
        let submissions = replications
          .iter()
          .map(|r| r.trace.jobs.len() as f64)
          .sum::<f64>()
          / REPS as f64;
        let ticks = total_used * MU1 / REPS as f64;
        let p95_median = quantile(evaluations.iter().map(|e| e.p95).collect(), 0.5);
        let p95_tail = quantile(evaluations.iter().map(|e| e.p95).collect(), 0.95);
        let mean = evaluations.iter().map(|e| e.mean).sum::<f64>() / REPS as f64;
        let peak = quantile(
          evaluations.iter().map(|e| e.peak_backlog as f64).collect(),
          0.95,
        );
        let peak_work = quantile(evaluations.iter().map(|e| e.peak_core_s).collect(), 0.95);
        let drain = quantile(evaluations.iter().map(|e| e.drain_s).collect(), 0.95);
        let workload_lower_bound = quantile(
          replications
            .iter()
            .map(|r| {
              r.trace
                .jobs
                .iter()
                .map(|j| if skip { j.skip_core_s } else { j.full_core_s })
                .sum::<f64>()
                / (CONTEST_S + sla)
            })
            .collect(),
          0.95,
        );
        let burst_lower_bound = quantile(
          replications
            .iter()
            .map(|r| {
              r.trace
                .jobs
                .iter()
                .filter(|j| j.arrival >= CONTEST_S - 1800.0)
                .map(|j| if skip { j.skip_core_s } else { j.full_core_s })
                .sum::<f64>()
                / (1800.0 + sla)
            })
            .collect(),
          0.95,
        );
        let expected_per_person: f64 = (0..5)
          .map(|z| {
            POPULATIONS[z] as f64
              * (0..4)
                .map(|t| scenario.cells[z][t].p * scenario.cells[z][t].c)
                .sum::<f64>()
          })
          .sum::<f64>()
          / scenario.n as f64;
        let assumptions = format!("N={};strata=U/B/M/S/E:{POPULATIONS:?};F9-U/B/M/S/E-layer-task-pxc;ability-task-correlated-runtime;weak-hard-fast-fail-and-early-zero;target-E[sub/person]={expected_per_person:.3};30m-final=19.78%;F10-fixed-NOIP2025-case-vector={:?};constraint-table-subtasks={NOIP_SUBTASKS:?};NOIP-overlap-priors=[8/20,11/25,7/25,10/20];active-subtask-testcase-union;internal-sale/tree-groups-and-query-queries-stay-within-one-sandbox-job;unique-testcases;end-to-end-runtime-effective-throughput;tick-derived-max-testcase-span;no-separate-runtime-overhead;FCFS-testcase-batch-fluid-backlog+parallel-span-completion;w_prefetch=2;drain-after-5h;no-N+1", scenario.n, scenario.cases);
        writeln!(writer, "{},{:.0},{:.6},{:.8},{:.6},{:.6},{:.6},{:.6},{:.2},{:.3},{:.6},{:.6},{:.6},{:.2},{:.3},{:.6},{:.6},{:.6},{},{:.6},\"{}\",{},{}", scenario.name, sla, cores, cores / 128.0, ci_lo, ci_hi, rep_lo, rep_hi, submissions, ticks, p95_tail, p95_median, mean, peak, peak_work, drain, workload_lower_bound, burst_lower_bound, if skip { "first-zero-approx" } else { "no-skip" }, 1.0 - total_used / total_full, assumptions, REPS, SEED)?;
      }
    }
    let cap_rate = replications
      .iter()
      .map(|r| r.trace.cap50_pairs as f64 / r.trace.attempted_pairs.max(1) as f64)
      .sum::<f64>()
      / REPS as f64;
    eprintln!(
      "{} complete: reps={}, workers={}, mean submissions={:.1}, cap50 pair rate={:.6}",
      scenario.name,
      REPS,
      workers,
      replications
        .iter()
        .map(|r| r.trace.jobs.len() as f64)
        .sum::<f64>()
        / REPS as f64,
      cap_rate
    );
  }
  writer.flush()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn skip_fraction_and_capacity_invariants_hold() {
    for task in 0..4 {
      let shape = TaskShape {
        cases: CENTRAL_CASES[task],
        subtasks: NOIP_SUBTASKS[task],
        shared_fraction: NOIP_SHARED_FRACTIONS[task],
      };
      for probability in [0.48, 0.70, 0.90, 0.98] {
        let fraction = expected_active_union_fraction(shape, probability);
        assert!((shape.shared_fraction..=1.0).contains(&fraction));
      }
    }
    let trace = make_trace(SCENARIOS[0], 999_999);
    assert!(!trace.jobs.is_empty());
    assert!(trace
      .jobs
      .windows(2)
      .all(|pair| pair[0].arrival <= pair[1].arrival));
    assert!(trace
      .jobs
      .iter()
      .all(|job| job.skip_core_s <= job.full_core_s && job.skip_core_s >= 0.0));
    assert!(required_cores(&trace, 120.0, true) >= required_cores(&trace, 600.0, true));
    assert!(required_cores(&trace, 300.0, false) >= required_cores(&trace, 300.0, true));
  }

  #[test]
  fn completion_uses_parallel_span_not_serial_batch_time() {
    let trace = Trace {
      jobs: vec![Job {
        arrival: 0.0,
        skip_core_s: 400.0,
        full_core_s: 400.0,
        precedence_s: 10.0,
      }],
      cap50_pairs: 0,
      attempted_pairs: 1,
    };
    assert!((evaluate(&trace, 100.0, false).p95 - 10.0).abs() < 1e-12);
    let trace = Trace {
      jobs: vec![
        Job {
          arrival: 0.0,
          skip_core_s: 400.0,
          full_core_s: 400.0,
          precedence_s: 10.0
        };
        2
      ],
      cap50_pairs: 0,
      attempted_pairs: 2,
    };
    assert!((evaluate(&trace, 100.0, false).p95 - 13.8).abs() < 1e-12);
  }
}
