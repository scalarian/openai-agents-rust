# Release

Use this page when you are preparing a release or making a change that affects the public docs surface.

## Release Checklist

1. update code and tests
2. update the affected docs pages
3. run docs maintenance scripts
4. run `cargo fmt --all`
5. run `cargo test --workspace`
6. confirm the README still points at the right docs entrypoints

## Docs Maintenance

After docs changes:

```bash
docs/scripts/check_links.sh
docs/scripts/generate_llms_exports.sh
```

## What To Update When Public API Changes

- `README.md`
- the relevant guide page under `docs/`
- the curated reference page under `docs/ref/`
- runnable examples if the change affects user-facing flow

## Read Next

- [ref/README.md](ref/README.md)
- [examples.md](examples.md)
