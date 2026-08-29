<!-- generated-policy: frozen -->

# `generated/` — committed, and not hand-editable

Everything in this directory is machine-written and **committed to version
control**. Do not hand-edit the adapters here. Change the source they come from
— the schema catalog under `schema/v1` — and re-run the generator. The generated
TypeScript and Dart must stay types-only.

Typical producers:

- [`flags-2-env`](https://github.com/flags-2-env/flags-2-env-cli) (`f2e generate`)
- [`api-docs` / `ridl`](https://github.com/oresoftware/api-docs) — route maps and clients
- interface adapters from the schema catalog (`schema/v1/flagcatalog.json`)

## Why the files are read-only on disk

After generation, artifact files are frozen with `chmod a-w` (0444). Directories
and this `README.md` stay writable so the generator can add and replace files.
Your editor will refuse the write, which is the point — it turns "I edited the
wrong file" into an error you see immediately rather than a diff you notice in
review.

**Git does not store this.** Git tracks only the executable bit (100644 vs
100755), so after `git clone` / `git checkout` the files come back writable. The
read-only bit is a local ergonomic guard; it is *not* what enforces the policy.
Restore it with the generator (`f2e generate`, `ridl generate`) or with:

```sh
scripts/freeze-generated.sh
python3 scripts/check-generated-contract.py --freeze --require-readonly
```

Do not `chmod u+w` and then commit a hand-edit. Change the source catalog
(`schema/v1/flagcatalog.json`, `.cli-flags.toml`, route map) and regenerate.

## What actually enforces the policy

CI, not the filesystem:

| Guard | Where | What it catches |
| --- | --- | --- |
| `check-generated-contract.py` | CI + pre-commit | a hand-edited or thawed file |
| regenerate-and-diff (`.generated-regen.sh`) | CI | committed output that no longer matches its source |
| `post-checkout` / `post-merge` hooks | your clone | re-freezes after every checkout |

Enable the hooks once per clone:

```sh
git config core.hooksPath .githooks
```

## Regenerating

Edit the **primary source** — `schema/v1/flagcatalog.json`, `.cli-flags.toml`,
the route map — then run the generator. Generators thaw, write, and re-freeze on
their own. If you are committing a regeneration, the pre-commit guard needs to be
told so:

```sh
REGEN=1 git commit -m "Regenerate interfaces from the updated schema catalog"
```

## JSON Schema (the contract)

The documents under `schema/` (and `json-schema/`, where present) are JSON Schema
2020-12 and are the interchange contract across Rust, TypeScript and Dart.
Compile-time types are generated *from* that catalog; the schema is an
independently derived description of the same contract, so disagreement means one
of them has drifted.

- Runtime `check_os_env` / `checkOsEnv` / `validate()` must pass on real
  payloads, not only on types that compile.
- Unit tests should feed **valid** and **invalid** instances (missing required
  keys, wrong types, extra properties); `scripts/seed-contract-fixtures.py`
  scaffolds the pairs under `tests/generated-contract/`.

```sh
f2e check-contract --config .cli-flags.toml --json env.fixture.json
```

## Gitignored trees

If a `generated/` folder is listed in `.gitignore`, its artifacts stay local and
the tree's policy is `ignored`, not `frozen`. Still commit the README so the
policy stays visible — `git add -f generated/README.md`, or a `.gitignore`
exception:

```
generated/*
!generated/README.md
```

(Do not ignore the directory node itself as `generated/` — that prevents the
`!README.md` exception from working.)
