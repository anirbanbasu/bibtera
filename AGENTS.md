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
