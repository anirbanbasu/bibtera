# BibTera: a BibTeX translator using the Tera templating engine

Parse BibTeX entries and generate output in Markdown (amongst other formats) using Python's [Jinja](https://github.com/pallets/jinja)-like [Tera templates](https://github.com/Keats/tera). The generated Markdown files can be used by static site generators, such as [Zola](https://github.com/getzola/zola).

> [!CAUTION]
> This project is in early development and may have breaking changes. Use with caution and report any issues you encounter.

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

BibTera provides two subcommands:

- `transform`: Parse entries and generate output files from a Tera template.
- `info`: Inspect what parsed fields are available for templating, without generating files.

### `transform`: generate files from a template

Use `transform` when you want BibTeX entries rendered into Markdown, HTML, JSON, or any other text format supported by your template file extension.

```bash
bibtera transform -i path/to/references.bib -o path/to/output -t path/to/template.md
```

Required options:

- `-i, --input`: Input BibTeX file.
- `-o, --output`: Output directory.
- `-t, --template`: Tera template file used for rendering.

Common filtering options:

- `--include key1,key2,...`: Process only selected BibTeX entry keys.
- `--exclude key1,key2,...`: Process all entries except selected BibTeX entry keys.

You can use either `--include` or `--exclude`, but not both in the same command.

Execution and safety options:

- `-n, --dry-run`: Preview which files would be generated, without writing files.
- `-f, --overwrite`: Overwrite existing files without confirmation prompts.
- `-v, --verbose`: Show detailed per-entry logs. Without this flag, BibTera shows progress and a final summary.

File naming options:

- `--file-name-strategy uuid7` (default): Stable, unique names derived from entry keys.
- `--file-name-strategy slugify`: Human-readable names derived from entry keys.

Single-output mode:

- `--single`: Render one combined output file from the full selected entry list (available in the template as `entries`) instead of one file per entry.

In `--single` mode, BibTera derives the output filename from the input and template names, so `--file-name-strategy` does not apply.

> [!IMPORTANT]
> Note that in this single file output mode, the template should be designed to handle the `entries` variable, which is a list of all selected entries. Each entry in this list will have the same fields available as in the per-entry output mode, but the template must iterate over `entries` to access them. For example, you might use a loop like `{% for entry in entries %} ... {% endfor %}` in your Tera template to render each entry's information.

### `info`: inspect available template data

Use `info` to understand what data fields you can reference in templates before running a transformation.

```bash
bibtera info --input path/to/references.bib
```

Useful options:

- `-i, --input`: Optional input BibTeX file.
- `--include key1,key2,...`: Show info only for selected BibTeX entry keys.
- `--exclude key1,key2,...`: Hide selected BibTeX entry keys from the info output.

Behavior of `info`:

- If entries are selected (through input and include/exclude filters), BibTera prints parsed key-value information for those entries.
- If no entries are selected, BibTera prints a key-value overview of supported entry types and fields available to templates.

For instance, the output displaying the available fields for the BibTeX entry type `@inproceedings`  looks like the following. Note that all `fields.*` are available for templating, but only a subset of them are guaranteed to be present for every entry of that type. Thus, `field.abstract` is not showed in the output below but it will be available if the BibTeX entry has the field specified.

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

> [!TIP]
> Start with `bibtera transform --help` or `bibtera info --help` to view command-specific details.
> You can find some example BibTeX files and Tera templates in the `examples/` directory of this repository to get started.
