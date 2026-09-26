# markdown-rs 1.0.0 (vendored)

Unmodified copy of [`markdown` 1.0.0](https://crates.io/crates/markdown) by Titus Wormer (MIT, see `license`),
except for the patch below. It is wired in through `[patch.crates-io]` in `rust/Cargo.toml`.

## Patch: GFM table body rows starting with `#`, `$`, `` ` ``, `~`, `*`, `_`, `e`, `i` or `{`

`src/construct/flow.rs` `start()` has byte fast paths that try one construct (heading, code fence/math,
thematic break, MDX ESM, MDX expression) and, on failure, jumped straight to `FlowBeforeContent`, skipping
the GFM table attempt. A table body row without a leading pipe that starts with one of those bytes
(e.g. `item | 1` or `` `x` | 1 ``) therefore ended the table and became a paragraph. micromark (the
JavaScript reference) keeps the row in the table. The five fast-path fallbacks now continue at
`FlowBeforeGfmTable`, which is what the generic chain does anyway: the constructs between the fast-path
target and the table cannot match those bytes. The bug is still present on upstream `main`.
