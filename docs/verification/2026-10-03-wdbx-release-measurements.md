# WDBX local release timing observations — 2026-10-03

**Current local measurements.** Five successful runs of the existing ABI CLI
HNSW workload, beginning `2026-10-03T07:49:54.843685+00:00`. Median across the five run-specific p50s:
**25.625 µs insert; 7.375 µs search**. This is a synthetic in-memory workload;
recall, durability and production throughput are not measured.

## All observed latency distributions

Values are microseconds per operation. Each run inserts 500 vectors and performs
200 repetitions of the same query. No run was discarded and no separate warmup
was performed.

| Run | Insert p50 | Insert p95 | Insert p99 | Search p50 | Search p95 | Search p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 25.708 | 35.958 | 62.125 | 7.333 | 7.500 | 7.833 |
| 2 | 25.625 | 36.542 | 60.167 | 7.417 | 7.667 | 12.584 |
| 3 | 25.208 | 34.167 | 59.667 | 7.375 | 7.584 | 8.250 |
| 4 | 25.625 | 35.708 | 60.750 | 7.417 | 7.750 | 9.291 |
| 5 | 25.584 | 34.125 | 61.041 | 7.375 | 7.750 | 13.333 |
| Median of run percentiles | 25.625 | 35.708 | 60.750 | 7.375 | 7.667 | 9.291 |

The last row takes a median separately for each column; it is not a percentile
of pooled samples. Raw logs retain total elapsed and average times as well.

## Reproduce the bounded workload

From the canonical ABI checkout, with its pinned nightly installed:

```sh
./tools/cargo.sh build --locked --release -p abi-cli
ABI_WDBX_PATH=:memory: ABI_WDBX_PERSIST=0 /usr/bin/time -l ./target/release/abi wdbx benchmark 500 < /dev/null
```

Repeat the second command five times. The retained driver bounds each invocation
to 60 seconds and runs no live store. Build exit: **0**, elapsed
**20.154 seconds**, excluded from operation timings.

- Mac17,2; 10 physical/logical cores; 24 GiB RAM; macOS 27.2.
- `rustc 1.100.0-nightly (0dfb098f3 2026-08-31)`; `cargo 1.100.0-nightly (e8cb624d5 2026-08-22)`.
- Release profile: thin LTO, one codegen unit, panic abort.
- Four-dimensional vectors `[i % 97, i % 31, 0, 0]`, IDs 1–500;
  query `[1, 0, 0, 0]`, k=10. HNSW seed 0, M=16, ef-construction=40,
  ef-search=32, maximum layers=4; scalar cosine distance.
- ABI HEAD `80dfe079ebe7413a6815c2d564be1f0aaa901267` plus captured dirty source;
  WDBX HEAD `e45410356ceb4e96f0dc26a83b02e1c18cf94860` plus captured dirty source.
- Binary SHA-256 `77863f103a9d0745c084bb647c63f3b50ea88f4e17dccb57dacd8419b99337c5`.
- Harness SHA-256 `26b3fc5cfc457762b6ff196691018988de382f7afe84807a865aeac08c569115`.

The fresh full gates preceded measurement: WDBX **647** Rust executions;
ABI **988** Rust executions and **144** Python tests. Complete source snapshots
(ABI 1,237 files, WDBX 750 files), HEAD and index remained unchanged through the
gates and measurement. Subsequent documentation updates do not change the measured
binary. The timing run does not replace the separate correctness suites.

## Resources and measurement limits

`/usr/bin/time -l` recorded whole-process peak RSS between **10,158,080 and
10,223,616 bytes**, including CLI startup. Its rounded wall-clock readings were
0.15 seconds for the first invocation and 0.01 seconds for each subsequent
invocation; these coarse whole-process values are distinct from Rust's operation
clocks. All raw time statistics are retained.

No other team build or render was scheduled during the five invocations. This is
a shared host, not an isolated benchmark machine: load averages were approximately
10.63 / 13.71 / 19.31 (1 / 5 / 15 minutes). No power, thermal or CPU-affinity
control is claimed.

The repeated query is cache-friendly. The existing harness discards its search
results; there is **no recall/accuracy oracle for this 500-vector fixture**.
These measurements say nothing about learned embeddings, model quality, realistic
query diversity, large corpora, durable-store/WAL/fsync latency, episode operations,
federation, accelerators or comparative product performance. No QPS or public
throughput claim is derived from them.

## Retained evidence

- [All observations, provenance and hashes](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/observations.json)
- [Bounded driver](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/run-release-benchmark.py)
- [Build receipt](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/build-result.json)
- [Raw run 1](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/run-1.log),
  [run 2](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/run-2.log),
  [run 3](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/run-3.log),
  [run 4](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/run-4.log),
  [run 5](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/release-benchmark/run-5.log)
- [ABI/WDBX frozen source snapshot](../../.superpowers/sdd/2026-10-03-ecosystem/evidence/resolution-abi-python314/abi-after.json)
- [Independent source review](../reviews/2026-10-03-memory-edge-resolution.md)

Evidence under `.superpowers/` is retained locally and ignored by Git. The source
was dirty and uncommitted; reproducing only the named HEADs omits those captured
changes. Earlier debug regression-guard measurements are historical observations
with a different profile and host load, not a release comparison baseline.
