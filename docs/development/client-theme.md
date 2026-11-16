# Client theme tokens (TypeScript)

`apps/client/src/theme/tokens.generated.ts` is the design tokens as a TypeScript module, generated
from the same Rust tokens the application and the docs site render with (`micold-core`'s `tokens`).

**Do not edit it by hand.** Change the tokens in Rust and regenerate.

## Regenerate

```sh
cargo run -p micold-core --bin micold-tokens-ts -- apps/client/src/theme/tokens.generated.ts
```

With no path argument the module is printed to stdout. The file is written atomically, so a failed
run leaves the committed file untouched.

## Drift check

`scripts/tests/tokens-ts-drift.test.sh` runs in the gate (`scripts/gate.sh`, CI). It regenerates the
module to a temporary file and compares it byte for byte with the committed one. On a difference,
or if the file is missing, it fails and prints the file path and the regeneration command above.
Commit the regenerated file with the token change.

The CSS side is described in [docs-site.md](docs-site.md): `micold-tokens-css` emits the site
stylesheet on every build and is never committed.
