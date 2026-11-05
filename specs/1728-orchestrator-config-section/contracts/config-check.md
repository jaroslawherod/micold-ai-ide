# Contract: `micold-daemon config check` for `[orchestrator]`

Exit 0 when valid, 2 when any error. Each error: `file:line:column: <message>` (existing format).

| Rule | Condition | Anchor | Message gist |
|---|---|---|---|
| O1 | table present, no `url` | `[orchestrator]` header | `url` is required |
| O2 | `url` not https / invalid | `url` value | must be an https URL with a host, no credentials or fragment |
| O3 | one of `cert_file`/`key_file` | the present key | names the missing key |
| O4 | token and pair both | `enrollment_token_file` key | give one: token enrolls, certificate is the result |
| O5 | neither token nor pair | `[orchestrator]` header | choose enrollment_token_file or cert_file+key_file |
| V4 | relative or empty path | the value | must be an absolute path |
| type | wrong value type | the value | serde type error |
| V15 (`--files`) | file missing or not a regular file | the value | error |
| V14 (`--files`) | `key_file`/token readable by others | the key | warning (host, ssh), error (container, pod) |

Resolved view block (absent without the table):

```
orchestrator.url = https://...
orchestrator.ca_file = /path (readable, 0600)
```
