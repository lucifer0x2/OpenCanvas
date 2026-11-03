# Generated benchmark outputs

Only this handwritten README is versioned here. Run outputs (logs, JSON/JSONL,
manifests, diagnostic scripts and source archives) are ignored and must not be
committed. Keep useful conclusions, limits and reproducible commands in reports.

The historical Task15/17 measurements remain in the human reports. Their raw
outputs were removed from the current tree by a follow-up cleanup commit; prior
commit `71e2142` still contains them. This does not shrink Git history.

Run the [current PostgreSQL harness](../README.md) or the
[Task17 lifecycle/benchmark commands](../../../docs/TASK17-REPORT.md) to generate
fresh outputs. Use a new destination per run, with an existing parent directory;
this README keeps the default `evidence/` parent present in clean checkouts.
Build before timing, run benchmarks sequentially, and keep credentials private.
Current-source runs do not reproduce older algorithms without their frozen source.
