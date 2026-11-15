# AttoLang contract projection (experimental)

This is a **downstream source-only AttoLang projection** of two existing interface shapes. It introduces **no new schema authority** or runtime JSON parser.

The authored `schema/v1/flagcatalog.tsp` and `schema/v1/flagcatalog.json` remain independent peer authorities, subject to TJSV parity admission. Their `FlagCatalog` contract requires nonempty string `id`, nonempty string `revision`, and a JSON-object `payload`; unknown root properties are forbidden. The schema permits whitespace-only identifiers, although some language runtimes apply stricter application validation. `Health` has typed `ok`, `service`, and `protocol` fields; `ok=false` is a valid shape, not an automatically accepted healthy response.

The .atto functions accept explicit scalar facts already established by a trusted host. They do not parse JSON, validate Unicode normalization, make HTTP calls, or grant permissions.

Run `node --test projections/atto/contract_model.test.mjs` for an **independent host-side oracle**, the checked-in positive/negative catalog fixtures, and a source/manifest drift gate. It does **not** compile or run AttoLang. Require independent TJSV conformance, real AttoLang compilation and negative execution, and consumer integration before shipping.
