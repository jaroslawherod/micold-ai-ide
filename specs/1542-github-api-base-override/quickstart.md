# Quickstart: #1542

1. `mise run test-core` : `pat` URL/base unit tests.
2. `scripts/build-lock.sh cargo test -p micold-client --test scoped_token_fake_api` : Part C steps 2, 3, 8, 10, 12 against the fake.
3. With a container runtime (and kind): `mise run image && mise run test-sandbox` : steps 4-8, 10 (step 11 in kind: follow-up #1760); without them the test reports skipped.
4. Confirm `specs/1002-sandbox-scoped-token/quickstart.md` "not live" table names the new tests.
5. Confirm the table still marks steps 1, 3 (real API), 7 (`/hooks` 403) manual and step 9 out of scope.
