# flags-2-env-interfaces

Data-only contracts for canonical CLI-flag-to-environment mapping used by ORESoftware runtimes.

`schema/v1/flagcatalog.tsp` and `schema/v1/flagcatalog.json` are independent, human-maintained TypeSpec and JSON Schema Draft 2020-12 peer authorities. Neither has precedence. `ORESoftware/typespec-json-schema-validator` (TJSV) must admit their structural and semantic convergence before downstream generated/runtime projections are trusted.

Generated TypeScript and Dart live under `generated/` and must stay types-only. Generated schemas and language/runtime artifacts are evidence/projections, never a third editable authority.
