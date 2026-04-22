# Requirements specification

The following specify the functional and non-functional requirements for the project.

## Functional requirements

The following functional requirements are being improved iteratively as the project evolves:

1. The application must be able to parse valid BibTeX entries from a given input file.
    1. When parsing author names, most BibTeX entries use the format "Last, First" or "Last, F N" for author names where "F N" represents the initial of the first name. However, some entries may use the format "First Last". The application should be able to handle both formats and convert them to a consistent format and expose the parts of the name for rendering to the template.
2. The application must be able to transform the parsed BibTeX entries into files in the desired output format using Tera templates. For each BibTeX entry, the application should generate a corresponding file in the specified output directory, using a user-specified Tera template for formatting. If there is only one BibTeX entry, the application should generate a single file.
    1. The file name of the generated file should be derived from the BibTeX entry's key. The file name must not contain any characters that are not allowed in file names (e.g., slashes, colons, etc.) and should ensure that the resulting file name is unique within the output directory to prevent overwriting existing files. Hint: use a hexadecimal representation of a keyed SHA-256 hash of the BibTeX entry's key to generate a unique file name.
3. The application must provide a command-line interface for users to specify input files, output directories, and template files.
    1. The CLI should expose no sub-commands.
    2. The CLI should expose the following options:
        1. `--input` or `-i`: Path to the input BibTeX file.
        2. `--output` or `-o`: Path to the output directory where the generated files will be saved.
        3. `--template` or `-t`: Path to the Tera template file used for formatting each file in the output directory.
        4. `--exclude`: A comma-separated list of BibTeX entry keys to exclude from processing.
        5. `--include`: A comma-separated list of BibTeX entry keys to include in processing. If specified, only these entries will be processed, and all others will be ignored. Either `--exclude` or `--include` can be used, but not both at the same time.
        6. `--dry-run` or `-n`: Perform a dry run without generating any files, but print the intended output file names and their corresponding BibTeX entry keys to the console.
        7. `--overwrite` or `-f`: Force overwrite of existing files in the output directory without prompting.
        8. `--verbose` or `-v`: Enable verbose logging for debugging purposes.
        9. `--help` or `-h`: Display usage information and exit.
        10. `--version` or `-V`: Display version information and exit.
4. The application must handle errors gracefully, providing informative error messages for issues such as invalid BibTeX entries, missing template files, and file I/O errors.
5. Since a Tera template can be used to generate any text-based output, the application should be flexible enough to accommodate different output formats. However, the application must not impose any file formats. Instead, the output file format should be derived from that of the template file. For example, if the template file has a `.md` extension, the generated output files should also have a `.md` extension.
    1. The application must not support generating binary output formats (e.g., PDF, DOCX, etc.) since Tera templates are designed for text-based output. The application should focus on generating text-based files (e.g., Markdown, HTML, plain text, etc.) that can be easily rendered by Tera templates.
    2. The application must not allow command-line options to specify the output file format directly (e.g., `--format md`) since the output file format should be determined by the template file's extension. Instead, users should be encouraged to use appropriate template files with the desired extensions to generate the corresponding output formats.
6. The application must be able to process large BibTeX files efficiently without excessive memory usage or long processing times.

## Non-functional requirements

1. The codebase must be well-structured and modular to facilitate maintainability and extensibility.
2. The application must have comprehensive test coverage to ensure reliability and facilitate future development.
3. The application must be documented with clear instructions for installation, usage, and contribution guidelines.
4. The application must be open-source and licensed under a permissive license (e.g., MIT License) to encourage community contributions.
5. The application must be designed with security best practices in mind, especially when handling file I/O and user input to prevent vulnerabilities such as path traversal and code injection.
6. The application must be performant, with optimizations for parsing and rendering to ensure a smooth user experience even with large datasets.
7. The application must be compatible with the latest stable version of Rust and should be regularly updated to maintain compatibility with new Rust releases and dependencies.
8. The application should use multithreading and/or asynchronous processing where appropriate to improve performance, especially when handling large inputs and outputs.
9. The application must be compatible with major operating systems (Windows, macOS, Linux).
