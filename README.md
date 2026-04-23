# BibTera: a BibTeX translator using the Tera templating engine

Parse BibTeX entries and generate output in Markdown (amongst other formats) using Tera templates. The generated Markdown files can be used by static site generators, such as [Zola](https://github.com/getzola/zola).

**Note**: This project is in early development and may have breaking changes. Use with caution and report any issues you encounter.

## Installation

The current preferred way is to download and compile the source from the HEAD of the main branch of BibTera using Cargo, Rust's package manager. You must have the [Rust toolchain installed](https://rust-lang.org/tools/install/). Run the following command in your terminal to install BibTera.

```bash
cargo install --git https://github.com/anirbanbasu/bibtera
```

## Usage

After installation, you can run BibTera from the command line. See the help message as follows:

```bash
bibtera --help
```

This will show you the available options and how to use them. The basic usage involves specifying the input BibTeX file, the output directory, and the template to use for generating the output.
