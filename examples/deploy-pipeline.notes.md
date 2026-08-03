# Deploy pipeline notes

This sidecar file renders in gridflow's **notes pane** (`Cmd+3`).

## Rollback procedure

1. `git revert` the offending merge commit
2. Re-run the pipeline from **CI**
3. If staging smoke tests fail twice, page the on-call

## Conventions

| Shape | Meaning |
|---|---|
| rectangle | action |
| diamond | decision |
| ellipse | terminal state |

```sh
# handy: watch the deployment
kubectl rollout status deploy/api --watch
```
