# Contract: the per-daemon opt-in

- Setting: `KubeSettings.github_tokens`, JSON key `github_tokens`, bool, default off, omitted when off.
- Form: one tick row in the Kubernetes daemon form, after **Allowed credentials**, label `GitHub tokens (stored in a Kubernetes Secret: anyone who can read Secrets in the namespace, and etcd, can read them)`; help line: only tokens whose repository matches a saved project of this daemon are sent (a token with no repository filter is sent too, though its reach may not include the project), replaced at each start, removed with the pod.
- Draft: `KubeDaemonDraft.github_tokens: bool`, `of(settings)` and `settings()` map it both ways.
- Effect: off = no Secret key, no `[[github.tokens]]` entry (today's behaviour). On = contract `pod-token-delivery.md`.
- Applies at the next start; the daemon's "applies on next start" notice covers it.
