# AGENTS.md

This file provides guidance to artificial intelligence coding agents such as Claude Code, GitHub Copilot, Open Code and so on when working with code in this repository.

## Project overview

Parse BibTeX entries and generate output in Markdown (amongst other formats) using Tera templates. The generated Markdown files can be used by static site generators, such as [Zola](https://github.com/getzola/zola).

## Tech stack

- Rust as the programming language.
- Cargo as the build system and package manager.
- [Tera](https://github.com/Keats/tera) as the templating engine.
- [BibLatex](https://github.com/typst/biblatex) as the library for parsing BibTeX entries.
- [Clap](https://github.com/clap-rs/clap) as the command-line argument parser.

## Essential commands

```bash
# Build the project in debug mode
cargo build

# Build the project in release mode
cargo build --release

# Format the code
cargo fmt

# Type check the code
cargo check

# Lint the code flagging warnings as errors
cargo clippy -- -D warnings

# Fix linting issues automatically where possible
cargo clippy --fix --allow-dirty --allow-staged

# Run the tests
cargo test

# Vulnerability scan, especially after adding new dependencies
osv-scanner scan source -r .
```

## Project structure overview

The core code is structured into the following modules within the `src` directory:

- `main.rs`: The entry point of the application, responsible for parsing command-line arguments and orchestrating the overall flow.
- `parser.rs`: Contains the logic for parsing BibTeX entries using the BibLatex library.
- `template.rs`: Contains the logic for rendering Tera templates with the parsed BibTeX entries.
- `utils.rs`: Contains utility functions that are used across the project.
- `cli.rs`: Contains the command-line interface definitions using Clap.
- `config.rs`: Contains runtime configuration validation and filter handling for CLI arguments.
- `latex.rs`: Contains logic for handling LaTeX-specific formatting and LaTeX substitutions.
- `lib.rs`: Exposes the library modules used by the binary and tests.

Outside the `src` directory, we have:

- `examples/`: A directory to store examples of inputs, templates and outputs.
- `tests/`: Integration and end-to-end test suites.
  - `integration_tests.rs`: Cross-module integration tests aligned with integration scenario IDs.
  - `end_to_end_tests.rs`: CLI end-to-end tests aligned with end-to-end scenario IDs.
  - `specifications/`: Machine-readable test scenario catalogues.
    - `integration-tests.json`: Integration scenario catalogue.
    - `end-to-end-tests.json`: End-to-end scenario catalogue.
    - `schemas/`: JSON Schema files for validating specification structures.
      - `scenario-catalogue.schema.json`: Schema for integration and end-to-end scenario files.
      - `fixture-catalogue.schema.json`: Schema for fixture catalogue files.
    - `test-data/`: Shared machine-readable test fixture definitions.
      - `fixtures.json`: Fixture catalogue referenced by integration and end-to-end specifications.
- `.github/workflows/`: CI workflows for Rust checks, security scanning, dependency review and publishing.
- `REQUIREMENTS.md`: Formal functional, non-functional, interface and test requirements.

## Platform support

- Standard Rust targets
- PyO3 for Python bindings (if implemented in the future)

## Requirements specification

Requirements are specified in `REQUIREMENTS.md`.

## Language of documentation

All function, method, variable names, comments, documentation, and user-facing messages must be written in British English (en-GB).
