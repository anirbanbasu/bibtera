# Requirements specification

The following specify the functional, non-functional and other requirements for the project.

**Requirements specification version**: _v2026-04-27-001-draft_.

## Functional requirements

The following functional requirements are being improved iteratively as the project evolves.

1. The application must be able to parse valid BibTeX entries from a given input file.
    1. When parsing author names, most BibTeX entries use the format "Last, First" or "Last, F N" for author names where "F N" represents the initial of the first name. However, some entries may use the format "First Last". The application should be able to handle both formats and convert them to a consistent format and expose the parts of the name for rendering to the template.
    2. There may be non-standard fields in the BibTeX entries, such as "abstract", "keywords", etc. The application should be able to parse these fields and make them available for rendering in the Tera templates, even if they are not part of the standard BibTeX specification.
    3. Sometimes, BibTeX field "month" may contain a non-numeric value (e.g., "January", "Feb", etc.) instead of a numeric representation (e.g., "1", "2", etc.). The application should be able to handle both cases and convert the month value to a consistent numeric representation. For both "month" and "day" fields, the values should be zero-prefixed for values less than 10, for rendering in the Tera templates.
    4. A BibTeX entry may contain a list of keywords in the "keywords" field. If it exists, the application should parse the keywords and expose them as an array of strings in the Tera templates through the `slugified_keywords` field, allowing users to access individual keywords as well as the full list. The application must slugify each keyword by replacing non-alphanumeric characters with hyphens.
    5. The application should exit immediately with an appropriate error message if the input BibTeX data cannot be parsed.
2. The application must be able to transform the parsed BibTeX entries into files in the desired output format using Tera templates. For each BibTeX entry, the application should generate a corresponding file in the specified output directory, using a user-specified Tera template for formatting. If there is only one BibTeX entry, the application should generate a single file.
    1. The file name of the generated file should be derived from the BibTeX entry's key. The file name must not contain any characters that are not allowed in file names (e.g., slashes, colons, etc.) and should ensure that the resulting file name is unique within the output directory to prevent overwriting existing files.
        1. Use a UUID7 representation with its 16-byte input from a SHAKE-128 hash of the BibTeX entry's key to generate a unique file name.
        2. Alternatively, slugify the BibTeX entry's key by replacing non-alphanumeric characters with underscores. The choice between these two approaches can be made configurable through a command-line option (i.e., `--file-name-strategy`, see CLI options).
    2. There is a special case where a template specification may contain Tera or Jinja-like syntax that will be parsed by a down stream template parser, such as the Zola static site generator. In this case, the application should not attempt to parse the template specification as a Tera template and instead should treat it as a literal string to be included in the output file. Tera already supports this using `{% raw %}...{% endraw %}` syntax, see [Tera documentation](https://keats.github.io/tera/docs/#raw). This applies to both one-line and multi-line ignored sections.
    3. The application must be able to output a special field called `raw_bibtex` that contains the raw BibTeX entry as a string, which can be used in the Tera templates for rendering the original BibTeX entry if needed.
    4. Usually, each BibTeX entry in the input file is expected to be transformed by a Tera template to a single output file in the output directory. However, in some cases, the Tera template may be designed such that it operates on the list of all parsed BibTeX entries rather than on individual entries. In this case, the application should generate a single output file that contains the rendered output for all the BibTeX entries in the input file, using the provided Tera template. This mode of operation can be specified by a command-line option, see below.
    5. The application should exit immediately with an appropriate error message if the transformation process fails for any reason, such as missing template files, file I/O errors, or issues with rendering the templates.
3. The application must provide a command-line interface (CLI) for users to interact with the application. The details are provided under the external interface requirements section below.
4. The application must handle errors gracefully, providing informative error messages for issues such as invalid BibTeX entries, missing template files, and file I/O errors.
5. Since a Tera template can be used to generate any text-based output, the application should be flexible enough to accommodate different output formats. However, the application must not impose any file formats. Instead, the output file format should be derived from that of the template file. For example, if the template file has a `.md` extension, the generated output files should also have a `.md` extension.
    1. The application must not support generating binary output formats (e.g., PDF, DOCX, etc.) since Tera templates are designed for text-based output. The application should focus on generating text-based files (e.g., Markdown, HTML, plain text, etc.) that can be easily rendered by Tera templates.
    2. The application must not allow command-line options to specify the output file format directly since the output file format should be determined by the template file's extension. Instead, users should be encouraged to use appropriate template files with the desired extensions to generate the corresponding output formats.
6. The application must be able to process large BibTeX files efficiently without excessive memory usage or long processing times.

## Non-functional requirements

1. The codebase must be well-structured and modular to facilitate maintainability and extensibility.
2. The application must have comprehensive test coverage to ensure reliability and facilitate future development.
3. The application must be documented with clear instructions for installation, usage, and contribution guidelines.
4. The application must be designed with security best practices in mind, especially when handling file I/O and user input to prevent vulnerabilities such as path traversal and code injection.
5. The application must be performant, with optimisations for parsing and rendering to ensure a smooth user experience even with large datasets.
6. The application must be compatible with the latest stable version of Rust and should be regularly updated to maintain compatibility with new Rust releases and dependencies.
7. The application may use multithreading and/or asynchronous processing where appropriate to improve performance, e.g., updating the CLI while working on file I/O.
    1. The application must process multiple BibTeX entries from a single input file sequentially to ensure that the output files are generated in a predictable order based on the input file.
8. The application must be compatible with major operating systems (Windows, macOS, Linux).

## External interface requirements

The supported external interfaces include: command-line interface (CLI).

### Command-line interface (CLI)

The CLI should expose two sub-commands: `transform` and `info`. The `transform` sub-command should be used for transforming BibTeX entries into files using Tera templates, while the `info` sub-command should be used for displaying information about the parsed BibTeX entries without generating any files. The purpose of the `info` sub-command is to tell the user about the information that can be used in the Tera templates for rendering the output files.

1. The `transform` sub-command should expose the following options:
    1. `--input` or `-i` (required): Path to the input BibTeX file.
    2. `--output` or `-o` (required): Path to the output directory where the generated files will be saved.
    3. `--template` or `-t` (required): Path to the Tera template file used for formatting each file in the output directory.
    4. `--exclude` (optional): A comma-separated list of BibTeX entry keys to exclude from processing.
    5. `--include` (optional): A comma-separated list of BibTeX entry keys to include in processing. If specified, only these entries will be processed, and all others will be ignored. Either `--exclude` or `--include` may be specified, but not both at the same time.
    6. `--dry-run` or `-n` (optional): Perform a dry run without generating any files, but print the intended output file names and their corresponding BibTeX entry keys to the console.
    7. `--overwrite` or `-f` (optional): Force overwrite of existing files in the output directory without prompting. If not specified, the application should ask for confirmation before generating files that already exist and print a warning message for each skipped file.
    8. `--file-name-strategy` (optional): Specify the strategy for generating output file names from BibTeX entry keys. Possible values are `uuid7` (default) and `slugify`. The `uuid7` strategy generates a unique file name using a UUID7 representation of a SHAKE-128 hash of the BibTeX entry's key, while the `slugify` strategy generates a file name by replacing non-alphanumeric characters in the BibTeX entry's key with underscores.
    9. `--single` (optional): This option specifies that the application should apply the template to the list of parsed BibTeX entries, which may be filtered by the `--exclude` or `--include` options, rather than to each entry individually. In this case, the application should expose a special variable called `entries` to the Tera template, which contains the list of entries that can be iterated over in the template. In this mode, the output file name should be derived from the input BibTeX file name and the template file name, rather than from the individual BibTeX entry keys. For example, if the input BibTeX file is `references.bib` and the template file is `template.md`, the output file could be named `references_template.md` or something similar that indicates it is generated from the input file and template. Hence, the `--single` option should ignore the default file naming strategy or the one specified by using `--file-name-strategy` option since the output file name is determined differently in this mode.
    10. `--verbose` or `-v` (optional): Enable additional verbose logging of the transformation process for debugging purposes. If this option is disabled, the application should log the progress of the transformation process, such as the number of entries processed and the number of files generated through a progress bar, without logging detailed information about each entry. If the verbose option is enabled, the application should log detailed information about each entry during the transformation process without displaying a progress bar. In either case, the application should output a summary of the transformation process at the end, including the total number of entries processed, the number of files generated, and the total time taken for the transformation process. Errors should be logged as is irrespective of the verbose option.
2. The `info` sub-command should expose the following options:
    1. `--input` or `-i` (optional): Path to the input BibTeX file.
    2. `--exclude` (optional): A comma-separated list of BibTeX entry keys to exclude from the information output. If specified, these entries will be excluded from the output, and all others will be included.
    3. `--include` (optional): A comma-separated list of BibTeX entry keys to include in the information output. If specified, only these entries will be included in the output, and all others will be ignored. Either `--exclude` or `--include` may be specified, but not both at the same time.
    If one or more BibTeX entries are selected through the options above, the `info` sub-command should parse those entries and display the parsed information as a key-value map of the parsed entries and their fields that are available to the Tera templates. If no entries can be selected or no options are provided, the `info` sub-command should display information as a key-value map of all BibTeX entry types and their corresponding fields that are available to the Tera templates.
3. The CLI should expose a global option `--help` or `-h`: Display usage information and exit.
4. The CLI should expose a global option `--version` or `-V`: Display version information and exit.

## Other requirements

This section describes test requirements and localisation requirements.

### Test requirements

The application should have the following testing setup, which is currently descriptive. In the future, the test specifications may refer to exact test data. When external files are needed for the tests, sample input files and templates present in the `examples` directory should be used. The input files in that directory are prefixed with `input_` and the template files are prefixed with `template_` to distinguish them.

1. Unit tests: The application should have comprehensive unit tests for all public functions and methods. These tests should cover all edge cases and error scenarios to ensure maximum coverage. These tests should be present in the `src` directory alongside the code they are testing, following Rust's convention for unit tests. Each test should be designed to test a specific function or method in isolation, using mock data and dependencies as needed to ensure that the tests are focused and reliable.
2. Integration tests: The application should have the following integration tests. These tests should be present as `integration_tests.rs` in the `tests` directory.
    1. Test that verify the parsing of sample BibTeX data.
    2. Tests to verify the normalisation of author names into a consistent representation including the structured given and family name parts.
    3. Tests to verify the normalisation of month and day fields into a consistent numeric representation with zero-prefixing for values less than 10.
    4. Tests to verify the parsing of keywords into a slugified array of strings.
    5. Tests to verify that the raw BibTeX entry is correctly exposed as a string for rendering in the Tera templates through the `raw_bibtex` field.
    6. Tests to verify the utilisation of sample Tera templates to generate the expected output files from sample BibTeX data.
    7. Tests to verify that the application exits immediately with appropriate error messages when input BibTeX or template files are invalid or missing.
3. End-to-end tests: The application should have the following end-to-end tests. The tests should be present as `end_to_end_tests.rs` in the `tests` directory.
    1. Tests that simulate a user running the CLI with various options for the `transform` sub-command and check the generated output files for correctness. These tests should contain the following scenarios.
        1. Running `transform` with a sample BibTeX file and a Tera template to check if the generated output files match the expected output.
        2. Running `transform` with the `--dry-run` option to check if the intended output file names and their corresponding BibTeX entry keys are printed to the console without generating any files.
        3. Running `transform` with the `--overwrite` option to check if existing files in the output directory are overwritten without prompting and if the warning message is printed for each skipped file when the option is not used.
        4. Running `transform` with the `--file-name-strategy` option to check if the output file names are generated according to the specified strategy (i.e., `uuid7` or `slugify`).
        5. Running `transform` with the `--single` option to check if a single output file is generated that contains the rendered output for all the BibTeX entries in the input file, and if the output file name is derived from the input BibTeX file name and the template file name as expected.
        6. Running `transform` with the `--verbose` option to check if detailed information about each entry is logged during the transformation process and if a summary of the transformation process is output at the end.
        7. Running `transform` with invalid input (e.g., malformed BibTeX file, missing template file, etc.) to check if the application exits immediately with an appropriate error message.
        8. Running `transform` with a real world input BibTeX file `examples/input_iclr2025_1k.bib` and the Tera template `examples/template_entry_single.json` to check if the generated output file is correct for a large dataset.
    2. Tests that simulate a user running the CLI with various options for the `info` sub-command and checks the output for correctness. These tests should contain the following scenarios.
        1. Running `info` without an input BibTeX file to check the output of available fields for all supported entry types.
        2. Running `info` with an input BibTeX file but without selecting any entries to check the output of available fields for the entry types present in the input file.
        3. Running `info` with an input BibTeX file and selecting some entries to check the output of parsed key-value information for those entries.
        4. Running `info` with a real world input BibTeX file `examples/input_iclr2025_1k.bib` to check the output of a random subset of parsed key-value information for all entries in that file.

### Localisation requirements

The application should be designed to support localisation of user-facing external interface strings in the future, allowing for the possibility of translating the user interface and error messages into different languages. The default language should be British English (en-GB), which is the only language supported at this time. All function, method, variable names, comments, documentation, and user-facing messages must be written in British English.

## Supplementary materials

None specified yet.
