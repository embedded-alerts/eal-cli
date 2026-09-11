<!-- generated-policy: frozen -->

# Generated files — read-only

Do **not** hand-edit files in this directory. They are produced by tooling such as:

- https://github.com/flags-2-env/flags-2-env (typical Dart path: `generated/dart/env.dart`)
- https://github.com/oresoftware/api-docs
- JSON Schema / OpenAPI / route-map generators in this repository

## Disk permissions

After generation, files here are frozen with `chmod a-w` (not writable). Directories
and this `README.md` stay writable so generators can replace files.

Git does **not** persist the write bit (only the executable bit). A fresh clone is
writable until you re-freeze:

```sh
find generated -type f ! -name 'README.md' ! -name 'readme.md' -exec chmod a-w {} +
```

To regenerate, change the **primary source** (`.cli-flags.toml`, route map, OpenAPI,
`schema/*.schema.json`, …) and re-run the generator. Preferred generators thaw,
write, then `chmod a-w` themselves.

## Gitignored trees

If `generated/` is in `.gitignore`, generated artifacts stay off VCS. Still commit
this `README.md` (`git add -f generated/README.md` or a `.gitignore` exception) so
the freeze policy is visible. Example exception:

```
generated/**
!generated/README.md
```

## Runtime and cross-language contracts

Generated files are downstream artifacts, never a third contract authority. Where a
contract is represented by independently authored TypeSpec and JSON Schema Draft
2020-12, their convergence must be admitted fail-closed with
`ORESoftware/typespec-json-schema-validator` (`tjsv`) before generated Rust, Dart,
TypeScript, WASM, SQL, or other runtime artifacts are promoted.

Use the repository's pinned TJSV invocation/CI workflow; do not replace either human
authority from the other or introduce a parallel custom parity validator. Runtime
fixture tests remain useful downstream evidence, but do not substitute for the TJSV
peer-authority admission gate.