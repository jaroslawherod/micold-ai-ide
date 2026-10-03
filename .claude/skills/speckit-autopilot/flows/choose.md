# Choosing the flow and the effort

Read at entry, before any unit. Every run follows one of four flows, at one effort level.

| Flow | For | File |
|---|---|---|
| **bug** (least effort) | broken behaviour that no spec covers, or whose spec stays as it is | [bug.md](bug.md) |
| **bugfix** (medium) | broken behaviour a spec describes wrongly or misses: the spec, plan or tasks must change | [bugfix.md](bugfix.md) |
| **feature** (full) | new behaviour | [feature.md](feature.md) |
| **chore** (no Spec Kit) | CI, build, tooling, tests, docs, dependency or config bumps: no behaviour a user sees | [chore.md](chore.md) |

Decide in this order. The command's own words win over labels; labels win over your reading.

1. **The user named it**, with an issue (`bug #702`, `#702 bugfix`, `chore low #587`) or with text
   (`bug: …`; `quick:`, `ci:`, `build:` and `docs:` mean chore): that flow and level, whatever the
   labels say. A named flow that plainly does not fit (`chore` for a change to behaviour): ask.
2. **The issue's labels** (`scripts/autopilot/issue.sh labels <n>`):

   | Label | Means |
   |---|---|
   | `flow:bug`, `flow:bugfix`, `flow:feature`, `flow:chore` | that flow, as if the user named it. It wins over `bug`, `enhancement` and `documentation`, which count only without a `flow:*` label |
   | `effort:high`, `effort:low` | the level, see the flow's file. With `bug` and no `flow:*`: `low` is the bug flow; `high` is the bugfix flow when a spec owns the behaviour, else ask |
   | `bug` | broken behaviour: bug or bugfix, never chore or feature. Which one: `effort:*`, else step 3 |
   | `enhancement` | new behaviour: feature flow |
   | `documentation` | chore flow |
   | none of these | step 3 |

3. **Your reading** of the prompt, the issue and `ls specs/`:
   - CI, build, tooling, tests or docs only → chore
   - broken behaviour, no spec covers it or the spec stays as is → bug
   - broken behaviour a spec describes wrongly or misses → bugfix
   - new behaviour → feature
4. **No level named or labelled:** the flow's default.

## When to ask

Labels and text agree, or one flow plainly fits: do not ask. Ask **exactly once**, with one
`AskUserQuestion` (header `Flow`), when:

- labels disagree with each other (two `flow:*`, both `effort:*`, `bug` with `enhancement`);
- a label disagrees with the text (`flow:bug` or `bug` on new behaviour, `enhancement` on a CI
  change);
- two flows fit, or a bug's owning spec is unclear.

Offer only the flows that fit, your pick first marked `(Recommended)`, each with one line on what
it costs and what it skips. Put the level in the same question's options when it is also open.

## Record it

- Pass the labels you read and the flow and level you chose to the first unit; it writes them in
  the ledger's **Input** and **Effort** lines.
- Write the choice back to the issue: [../tasks/issue.md](../tasks/issue.md).

Next: the chosen flow's file. A unit that later returns `NEXT:`: [switch.md](switch.md).
