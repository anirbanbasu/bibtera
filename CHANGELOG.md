# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/) and this project adheres to [Semantic Versioning](https://semver.org/).

## [unreleased]

### Added

- An example template, `examples/template_entry_zola.md`, that generates Zola pages with TOML front matter and passes a Zola 0.23 component call through to Zola using `{% raw %}`.
- End-to-end tests that build the generated Markdown with Zola 0.23, checking that pages are built, components are resolved and special characters in titles remain valid TOML. These tests are skipped when `zola` is not installed, unless `BIBTERA_REQUIRE_ZOLA` is set.
- An end-to-end regression test that HTML-significant characters in field values are never autoescaped.

### Changed

- The Rust workflow installs Zola 0.23.6, verified by its SHA-256 digest, so that the Zola end-to-end tests run in continuous integration.
- Documented in the README that BibTera uses Tera 2, how to migrate Tera 1 templates, and how Zola 0.23 components replace shortcodes.
- Upgraded dependencies.
- Bumped the patch version to 0.1.3.

### Deprecated

- None documented yet.

### Removed

- None documented yet.

### Fixed

- None documented yet.

### Security

- None documented yet.

## [0.1.2] - 2026-06-20

### Added

- Mutually exclusive `--exclude-type` and `--include-type` options to filter the BibTeX entries to be processed based on their types.

### Changed

- Updated the requirements specification.
- Upgraded dependencies.

### Deprecated

- None documented yet.

### Removed

- None documented yet.

### Fixed

- None documented yet.

### Security

- None documented yet.

## [0.1.1] - 2026-04-30

### Added

- A `latex_substitute` Tera template helper (available as both function and filter) that converts LaTeX markup to plain Unicode text.

### Changed

- Updated the requirements specification.
- Upgraded dependencies.

## [0.1.0] - 2026-04-27

### Added

- The functionality to transform BibTeX entries into different output formats using Tera templates.
- The ability to filter the BibTeX entries to be processed based on their keys.
- The ability to output either one file per BibTeX entry or a single combined file for all entries.

### Security

- There is a reported vulnerability with unknown impact in a downstream dependency, `paste`, which is no longer maintained. See: [RUSTSEC-2024-0436](https://osv.dev/RUSTSEC-2024-0436). This vulnerability cannot be addressed until the dependency -- `biblatex` -- using `paste` changes it to a maintained alternative. However, as of now, there is no such plan as discussed in the [issue 99](https://github.com/typst/biblatex/issues/99).


[unreleased]: https://github.com/anirbanbasu/bibtera/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/anirbanbasu/bibtera/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/anirbanbasu/bibtera/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/anirbanbasu/bibtera/compare/v0.0.1...v0.1.0
