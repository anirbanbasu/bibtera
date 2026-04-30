# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/) and this project adheres to [Semantic Versioning](https://semver.org/).

## [unreleased]

### Added

- A `latex_substitute` Tera template helper (available as both function and filter) that converts LaTeX markup to plain Unicode text.

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

## [0.1.0] - 2026-04-27

### Added

- The functionality to transform BibTeX entries into different output formats using Tera templates.
- The ability to filter the BibTeX entries to be processed based on their keys.
- The ability to output either one file per BibTeX entry or a single combined file for all entries.

### Security

- There is a reported vulnerability with unknown impact in a downstream dependency, `paste`, which is no longer maintained. See: [RUSTSEC-2024-0436](https://osv.dev/RUSTSEC-2024-0436). This vulnerability cannot be addressed until the dependency -- `biblatex` -- using `paste` changes it to a maintained alternative. However, as of now, there is no such plan as discussed in the [issue 99](https://github.com/typst/biblatex/issues/99).


[unreleased]: https://github.com/anirbanbasu/bibtera/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/anirbanbasu/bibtera/compare/v0.0.1...v0.1.0
