# BibTera: a BibTeX translator using the Tera templating engine

[![CodeQL Advanced](https://github.com/anirbanbasu/bibtera/actions/workflows/codeql.yml/badge.svg)](https://github.com/anirbanbasu/bibtera/actions/workflows/codeql.yml) [![Markdown Lint](https://github.com/anirbanbasu/bibtera/actions/workflows/md-lint.yml/badge.svg)](https://github.com/anirbanbasu/bibtera/actions/workflows/md-lint.yml) [![Rust tests](https://github.com/anirbanbasu/bibtera/actions/workflows/rust.yml/badge.svg)](https://github.com/anirbanbasu/bibtera/actions/workflows/rust.yml) [![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/anirbanbasu/bibtera/badge)](https://scorecard.dev/viewer/?uri=github.com/anirbanbasu/bibtera) ![crates.io](https://img.shields.io/crates/v/bibtera.svg)

BibTera parses BibTeX entries and generates outputs in text formats, such as Markdown amongst others, using [Jinja](https://github.com/pallets/jinja)-like [Tera](https://github.com/Keats/tera) templates. Static site generators, such as [Zola](https://github.com/getzola/zola), can use the generated Markdown files to render bibliography content.

> [!CAUTION]
> BibTera is still in an early stage of development. It may have bugs making it unsuitable for production use yet. Use it at your own risk and please report any issues you encounter.

## Installation

The current preferred way is to download and compile the source from the HEAD of the main branch of BibTera using Cargo, Rust's package manager. You must have the [Rust toolchain installed](https://rust-lang.org/tools/install/). Run the following command in your terminal to install BibTera.

```bash
cargo install --git https://github.com/anirbanbasu/bibtera
```

You can also install the latest released version from [crates.io](https://crates.io/crates/bibtera) using the following command.

```bash
cargo install bibtera
```

Alternatively, you can install BibTera using [Homebrew](https://brew.sh/) on macOS or Linux with the following command.

```bash
brew install anirbanbasu/tap/bibtera
```

## Usage

After installation, you can run BibTera from the command line. See the help message as follows:

```bash
bibtera --help
```

This will show you the available options and how to use them. The basic usage involves specifying the input BibTeX file, the output directory, and the template to use for generating the output.

BibTera provides two subcommands as follows.

- `transform`: Parse bibliography entries from a BibTeX file and generate output files using a Tera template.
- `info`: Display the available parsed or parsable bibliography fields are available for templating, without generating output files.

### `transform`: generate files from a template

Use `transform` to render BibTeX entries as Markdown, HTML, JSON, or any other text format.

```bash
bibtera transform -i path/to/references.bib -o path/to/output -t path/to/template.md
```

Required options are shown below.

- `-i, --input`: Input BibTeX file.
- `-o, --output`: Output directory.
- `-t, --template`: Tera template file used for rendering.

Common filtering options are as follows.

- `--include key1,key2,...`: Process only selected BibTeX entry keys.
- `--exclude key1,key2,...`: Process all entries except selected BibTeX entry keys.

You can use either `--include` or `--exclude`, but not both in the same command.

Execution and safety options include the following.

- `-n, --dry-run`: Preview the files to be generated, without writing files.
- `-f, --overwrite`: Overwrite existing files without confirmation prompts.
- `--latex-substitution-map path/to/substitutions.json`: Load a custom LaTeX substitution map that extends and/or overrides built-in defaults used by the `latex_substitute` template helper.
- `-v, --verbose`: Show detailed per-entry logs. Without this flag, BibTera shows progress and a final summary.

There is the following file naming option.

- `--file-name-strategy uuid7` (default): Stable, unique names derived from entry keys.
- `--file-name-strategy slugify`: Human-readable names derived from entry keys.

Also, a single-output mode.

- `--single`: Render one combined output file from the full selected entry list (available in the template as `entries`) instead of one file per entry.

In `--single` mode, BibTera derives the output filename from the input and template names, so `--file-name-strategy` does not apply.

When using `--latex-substitution-map`, the custom JSON map is merged on top of the built-in defaults shipped in `examples/substitution_map_default.json`. Each key must be a LaTeX token and each value must be the plaintext replacement.

```json
{
  "\\textemdash": "--",
  "\\ss": "ß"
}
```

> [!IMPORTANT]
> Note that in this single file output mode, the template should be designed to handle the `entries` variable, which is a list of all selected entries. Each entry in this list will have the same fields available as in the per-entry output mode, but the template must iterate over `entries` to access them. For example, you might use a loop like `{% for entry in entries %} ... {% endfor %}` in your Tera template to render each entry's information.

### `info`: inspect available template data

Use `info` to understand what data fields you can reference in templates before running a transformation.

```bash
bibtera info --input path/to/references.bib
```

Useful options are as follows.

- `-i, --input`: Optional input BibTeX file.
- `--include key1,key2,...`: Show info only for selected BibTeX entry keys.
- `--exclude key1,key2,...`: Hide selected BibTeX entry keys from the info output.

The output of `info` depends on whether an input file is provided and whether entries are selected through filters.

- If entries are selected (through input and include/exclude filters), BibTera prints parsed key-value information for those entries.
- If no entries are selected, BibTera prints a key-value overview of supported entry types and fields available to templates.

For instance, the output of `info` without an input BibTeX file displaying the available fields for the BibTeX entry type `@inproceedings`  looks like the following. Note that all `fields.*` are available for templating, but only a subset of them are guaranteed to be present for every entry of that type. Thus, `field.abstract` is not showed in the output below but it will be available if the BibTeX entry has the field specified.

```json
"inproceedings": {
    "author_parts": "array<{first:string,last:string,full:string}>",
    "authors": "array<string>",
    "entry_type": "string",
    "fields": "map<string,string>",
    "fields.author": "string",
    "fields.booktitle": "string",
    "fields.month": "string",
    "fields.pages": "string",
    "fields.publisher": "string",
    "fields.title": "string",
    "fields.year": "string",
    "key": "string",
    "raw_bibtex": "string",
    "slugified_keywords": "array<string>",
    "title": "string",
    "year": "string|null"
  }
```

### Global options

- `-h, --help`: Show help.
- `-V, --version`: Show installed version.

## Writing templates

> [!TIP]
> You can find some example BibTeX files and Tera templates in the `examples/` directory of this repository to get started.

Following the [documentation of the Tera template engine](https://keats.github.io/tera/docs/), you can learn about the basic syntax for writing Tera templates. The template you write will depend on the specific type of output you want to generate. Thus, for instance, a template to generate vanilla Markdown will be different from a template to generate the Markdown for a theme for the [Zola static site generator](https://github.com/getzola/zola), which has its own syntax for front matter and content.

> [!TIP]
> What if the template syntax conflicts with the syntax of the output format you want to generate? For instance, this will happen if the output you want to generate also uses Tera or Tera-like syntax to be processed by something else down the pipeline, such as Zola. Use the `{% raw %}...{% endraw %}` syntax, see [Tera documentation](https://keats.github.io/tera/docs/#raw), to escape the Tera syntax in the template and have it appear verbatim in the output.

### LaTeX substitution helper

BibTera registers a helper named `latex_substitute` in templates as both a function and a filter. It applies LaTeX-to-plaintext substitutions using the built-in map, optionally overridden via `--latex-substitution-map`.

Function usage:

```tera
{{ latex_substitute(value=title) }}
{{ latex_substitute(text=fields.note) }}
```

Filter usage:

```tera
{{ title | latex_substitute }}
{{ fields.abstract | latex_substitute }}
```

Formatting commands such as `\textbf{...}` and nested forms such as `\textbf{bold \emph{and italic} text}` are unwrapped to plain text before substitutions are applied. Only braced formatting-command forms are unwrapped. If a recognised formatting command is not followed by a braced argument, it is preserved verbatim.

> [!NOTE]
> Some spacing and token-shape normalisation may occur upstream during BibTeX parsing, before template helpers are evaluated. As a result, exact whitespace and token boundaries in rendered output can vary with BibTeX authoring style and parser behaviour. Templates should avoid relying on fragile assumptions about exact inter-token spacing unless that spacing is explicitly encoded in the source data.
> If spacing is semantically important in your output, encode it explicitly in the BibTeX source (for example, with explicit spacing commands) rather than relying on implicit inter-token whitespace.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

## Licence

This project is licensed under the [MIT License](https://choosealicense.com/licenses/mit/). See [LICENSE](LICENSE) for the full licence text.
