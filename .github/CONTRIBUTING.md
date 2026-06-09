# BEST PRACTICES

## 1. Project Goals

This document defines the working standards for aperture. The goals are:

- keep the codebase easy to understand;
- avoid preventable merge conflicts (use issues effectively);
- make review lightweight but consistent;
- keep main stable;
- lint consistently;
- document everything well.

## 2. Rust Code Standards

All code should pass:

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo build`

General style:

- prefer clarity in naming;
- keep modules cohesive and split up if they get too heavy;
- prefer explicit error types.

## 3. Dependencies

Avoid dependency hell with these simple tricks:

- Is the crate not maintained frequently/well/in recent history? Avoid
- Is it not widely used? Avoid (unless you really need it)
- Is the license incompatible with MIT? Avoid
- Is it not worth the added compile time? Avoid

## 4. Testing

Every nontrivial feature should include tests. Bug fixes should include a
regression test. Before opening a PR, the branch should pass the code standards.

## 5. Branching Model

main is always stable.

Branches should be short-lived and tagged based on their type with the slash
prefix model:

```
feat/<name>
fix/<name>
refactor/<name>
docs/<name>
test/<name>
```

## 6. Default Workflow

```
pull main
create feature branch
make small commits
run fmt/clippy/test
open pr
review
rebase if needed
squash merge
delete branch
```
