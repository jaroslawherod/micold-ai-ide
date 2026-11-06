# Data model: #1542

- **ApiBase** (newtype over `String`, `micold-core::scoped_credential::pat`): `Default` = `https://api.github.com`; `new(s)` trims whitespace and trailing `/`, empty becomes default. `probe_url(repo)` = `{base}/repos/{owner}/{name}`; `reach_url()` = `{base}/user/repos?visibility=private&per_page=100`. Not serialized, not stored.
- **FakeGithub** (test support): listener on `127.0.0.1:0`; routes keyed by path, each a canned `Reply` (status, headers incl. `github-authentication-token-expiration`, body); records `(path, Authorization)` per request; `base() -> ApiBase`.
