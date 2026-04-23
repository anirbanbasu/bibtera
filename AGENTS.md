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

The project is structured into the following modules within the `src` directory:

- `main.rs`: The entry point of the application, responsible for parsing command-line arguments and orchestrating the overall flow.
- `parser.rs`: Contains the logic for parsing BibTeX entries using the BibLatex library.
- `template.rs`: Contains the logic for rendering Tera templates with the parsed BibTeX entries.
- `utils.rs`: Contains utility functions that are used across the project.
- `cli.rs`: Contains the command-line interface definitions using Clap.
- `config.rs`: Contains the configuration management logic, if needed in the future.
- `lib.rs`: Contains the core library code, if the project is structured as a library and binary.

Outside the `src` directory, we have:

- `templates/`: A directory to store Tera template files.
- `examples/`: A directory to store example BibTeX files and generated Markdown outputs.
- `tests/`: A directory to store integration tests, if needed in the future.

## Platform support

- Standard Rust targets
- PyO3 for Python bindings (if implemented in the future)

## Requirements specification

Requirements are specified in REQUIREMENTS.md.
