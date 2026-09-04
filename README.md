# build-logs — CI run log archive (branch `build-logs`)

Every `CI-v1` workflow run auto-pushes its logs here so we can analyze
Titan vs primecount over time without digging through Actions UI retention.

## Layout

```text
runs/<RUN_ID>-<RUN_NUMBER>/<job>.log   # raw captured output per job
runs/<RUN_ID>-<RUN_NUMBER>/meta.json   # sha, branch, runner, ncpu, timestamp
latest/<job>.log                       # overwritten pointer to newest run
```

## Retention

- Raw GitHub Actions logs expire (default 90d); this branch is the durable mirror.
- Old `runs/` entries pruned manually when the branch gets heavy (shallow mirror, no source).
- Never merge into `main` / `CI-v1`. Read-only analysis + `git log -- runs/`.
