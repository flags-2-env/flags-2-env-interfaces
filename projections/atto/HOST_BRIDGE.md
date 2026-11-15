# AttoLang host FlagCatalog bridge (experimental)

`decode_catalog_for_atto` adds a typed, at-most-1 MiB JSON admission
boundary for the existing FlagCatalog root. It rejects missing, duplicate
and unknown root keys, malformed object payloads and empty names. Whitespace
names remain valid because the authored JSON Schema uses `minLength: 1`.

This is a Rust host decoder for later AttoLang FFI, **not** a native AttoLang
parser or a new schema authority. Nested payload contracts and runtime
permissions require separate validation.

Run `cargo test --all-targets` and `node --test projections/atto/*.test.mjs`.
The Node model does not execute Rust or AttoLang. The existing TJSV peer
authority gate must pass separately before release.
