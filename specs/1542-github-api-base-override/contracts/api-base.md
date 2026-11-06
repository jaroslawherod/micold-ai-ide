# Contract: API base parameter

```rust
pub struct ApiBase(String);            // Default = "https://api.github.com"
impl ApiBase { pub fn new(s: &str) -> Self; fn probe_url(&self, &GithubRepo) -> String; fn reach_url(&self) -> String }
pub fn check(transport, &ApiBase, repo, token, now) -> Result<UnixTime, NoCredentialReason>;
pub fn check_added(transport, &ApiBase, token, probe, now) -> Result<Checked, NoCredentialReason>;
```

- Default base yields exactly today's URLs (`REACH_URL` string and `https://api.github.com/repos/o/r`).
- A token is sent only to URLs built from the base passed; a malformed base surfaces as `SourceUnreachable` from the transport.
- No caller in shipped code builds a non-default base; the daemon helper keeps `MICOLD_GIT_CREDENTIAL_BASE` unchanged.
