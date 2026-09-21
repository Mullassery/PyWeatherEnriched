## What does this change?

<!-- One or two sentences: what does this PR do, and why? -->

## How was this tested?

<!--
Be specific and honest. "Works on my machine" untested claims aren't
useful — say exactly what you ran and what the output was, e.g.:
- `cargo test --lib` → N passed
- `pytest tests/ -v` → N passed
- Manually ran `enricher.enrich_row(...)` against real Nominatim/Open-Meteo and checked the output
-->

## Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --lib -- -D warnings` passes
- [ ] `cargo test --lib` passes
- [ ] `pytest tests/ -v` passes
- [ ] Updated `README.md` / `ROADMAP_HONEST.md` / `CHANGELOG.md` if this
      changes what they claim about the project's status
- [ ] No fabricated/placeholder data introduced — unimplemented paths
      fail loudly with a clear error instead
