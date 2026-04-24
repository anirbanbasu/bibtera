# Requirements specification

The following specify the functional and non-functional requirements for the project.

**Requirements specification version**: _v2026-04-24-003-with-todos_.

## Functional requirements

The following functional requirements are being improved iteratively as the project evolves:

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
    4. The application should exit immediately with an appropriate error message if the transformation process fails for any reason, such as missing template files, file I/O errors, or issues with rendering the templates.
3. The application must provide a command-line interface for users to specify input files, output directories, and template files.
    1. The CLI should expose two sub-commands: `transform` and `info`. The `transform` sub-command should be used for transforming BibTeX entries into files using Tera templates, while the `info` sub-command should be used for displaying information about the parsed BibTeX entries without generating any files. The purpose of the `info` sub-command is to tell the user about the information that can be used in the Tera templates for rendering the output files.
    2. The `transform` sub-command should expose the following options:
        1. `--input` or `-i` (required): Path to the input BibTeX file.
        2. `--output` or `-o` (required): Path to the output directory where the generated files will be saved.
        3. `--template` or `-t` (required): Path to the Tera template file used for formatting each file in the output directory.
        4. `--exclude` (optional): A comma-separated list of BibTeX entry keys to exclude from processing.
        5. `--include` (optional): A comma-separated list of BibTeX entry keys to include in processing. If specified, only these entries will be processed, and all others will be ignored. Either `--exclude` or `--include` may be specified, but not both at the same time.
        6. `--dry-run` or `-n` (optional): Perform a dry run without generating any files, but print the intended output file names and their corresponding BibTeX entry keys to the console.
        7. `--overwrite` or `-f` (optional): Force overwrite of existing files in the output directory without prompting. If not specified, the application should ask for confirmation before generating files that already exist and print a warning message for each skipped file.
        8. `--file-name-strategy` (optional): Specify the strategy for generating output file names from BibTeX entry keys. Possible values are `uuid7` (default) and `slugify`. The `uuid7` strategy generates a unique file name using a UUID7 representation of a SHAKE-128 hash of the BibTeX entry's key, while the `slugify` strategy generates a file name by replacing non-alphanumeric characters in the BibTeX entry's key with underscores.
        9. `--verbose` or `-v` (optional): Enable additional verbose logging of the transformation process for debugging purposes. If this option is disabled, the application should log the progress of the transformation process, such as the number of entries processed and the number of files generated through a progress bar, without logging detailed information about each entry. If the verbose option is enabled, the application should log detailed information about each entry during the transformation process without displaying a progress bar. In either case, the application should output a summary of the transformation process at the end, including the total number of entries processed, the number of files generated, and the total time taken for the transformation process. Errors should be logged as is irrespective of the verbose option.
    3. The `info` sub-command should expose the following options:
        1. `--input` or `-i` (optional): Path to the input BibTeX file.
        2. `--exclude` (optional): A comma-separated list of BibTeX entry keys to exclude from the information output. If specified, these entries will be excluded from the output, and all others will be included.
        3. `--include` (optional): A comma-separated list of BibTeX entry keys to include in the information output. If specified, only these entries will be included in the output, and all others will be ignored. Either `--exclude` or `--include` may be specified, but not both at the same time.
        If one or more BibTeX entries are selected through the options above, the `info` sub-command should parse those entries and display the parsed information as a key-value map of the parsed entries and their fields that are available to the Tera templates. If no entries can be selected or no options are provided, the `info` sub-command should display information as a key-value map of all BibTeX entry types and their corresponding fields that are available to the Tera templates.
    4. The CLI should expose a global option `--help` or `-h`: Display usage information and exit.
    5. The CLI should expose a global option `--version` or `-V`: Display version information and exit.
4. The application must handle errors gracefully, providing informative error messages for issues such as invalid BibTeX entries, missing template files, and file I/O errors.
5. Since a Tera template can be used to generate any text-based output, the application should be flexible enough to accommodate different output formats. However, the application must not impose any file formats. Instead, the output file format should be derived from that of the template file. For example, if the template file has a `.md` extension, the generated output files should also have a `.md` extension.
    1. The application must not support generating binary output formats (e.g., PDF, DOCX, etc.) since Tera templates are designed for text-based output. The application should focus on generating text-based files (e.g., Markdown, HTML, plain text, etc.) that can be easily rendered by Tera templates.
    2. The application must not allow command-line options to specify the output file format directly (e.g., `--format md`) since the output file format should be determined by the template file's extension. Instead, users should be encouraged to use appropriate template files with the desired extensions to generate the corresponding output formats.
6. The application must be able to process large BibTeX files efficiently without excessive memory usage or long processing times.

## Non-functional requirements

1. The codebase must be well-structured and modular to facilitate maintainability and extensibility.
2. The application must have comprehensive test coverage to ensure reliability and facilitate future development.
3. The application must be documented with clear instructions for installation, usage, and contribution guidelines.
4. The application must be designed with security best practices in mind, especially when handling file I/O and user input to prevent vulnerabilities such as path traversal and code injection.
5. The application must be performant, with optimisations for parsing and rendering to ensure a smooth user experience even with large datasets.
6. The application must be compatible with the latest stable version of Rust and should be regularly updated to maintain compatibility with new Rust releases and dependencies.
7. The application should use multithreading and/or asynchronous processing where appropriate to improve performance, especially when handling large inputs and outputs.
    1. The application must process multiple BibTeX entries sequentially to ensure that the output files are generated in a predictable order based on the input file.
8. The application must be compatible with major operating systems (Windows, macOS, Linux).

## External interface requirements

TODO: Define any external interfaces (e.g., UIs, APIs, file formats, etc.) that the application will interact with. Some of the functional requirements above already specify interactions with the command-line interface and file I/O may need to be specified here instead.

## Other requirements

TODO: Define any other requirements that do not fit into the categories above, such as legal or regulatory requirements, ethical considerations, etc. Also, include internationalisation, design contraints, database requirements, etc. if applicable.

## Supplementary materials

TODO: Include a glossary of terms, use-case diagrams and workflow models, data models (ER diagrams), traceability matrix linking requirements to design and tests, etc. as needed to support the requirements specification.
