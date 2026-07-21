//! End-to-end tests for the BibTeX converter CLI.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use bibtera::config::FileNameStrategy;
use bibtera::utils;
use serde_json::Value;

fn repo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_bibtera").expect("CARGO_BIN_EXE_bibtera must be set"),
    )
}

fn examples_dir() -> PathBuf {
    repo_dir().join("examples")
}

fn unique_test_dir(stem: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();

    repo_dir()
        .join("target")
        .join("tmp")
        .join(format!("{}_{}", stem, nonce))
}

fn cleanup_repo_root_output_artifacts() {
    for name in ["output.md", "entries_output.md"] {
        let _ = fs::remove_file(repo_dir().join(name));
    }
}

fn run_bibtera(args: &[&str], stdin: Option<&str>) -> Output {
    cleanup_repo_root_output_artifacts();

    let mut command = Command::new(binary_path());
    command
        .args(args)
        .current_dir(repo_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().expect("spawn bibtera binary");

    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin handle")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait for process output");
    cleanup_repo_root_output_artifacts();
    output
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn read_json_file(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json output"))
        .expect("parse json output")
}

#[test]
fn e2e_transform_basic_001_generates_expected_output_files() {
    let output_dir = unique_test_dir("e2e_transform_basic");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
        ],
        None,
    );

    assert!(output.status.success());
    assert!(stderr_text(&output).contains("Summary: processed 6 entries, generated 6 files"));

    let smith_output = output_dir.join("smith2020machine.md");
    assert!(smith_output.exists());
    let smith_rendered = fs::read_to_string(&smith_output).expect("read smith output");
    assert!(smith_rendered.contains("# Machine Learning for Natural Language Processing"));
    assert!(smith_rendered.contains("<!-- citation: smith2020machine -->"));

    let file_count = fs::read_dir(&output_dir).expect("list output dir").count();
    assert_eq!(file_count, 6);

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_dry_run_001_reports_planned_outputs_without_writing_files() {
    let output_dir = unique_test_dir("e2e_transform_dry_run");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--dry-run",
        ],
        None,
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("smith2020machine -> smith2020machine.md"));
    assert!(stdout.contains("carol2020thesis -> carol2020thesis.md"));
    assert!(!output_dir.exists());
}

#[test]
fn e2e_transform_overwrite_001_honours_skip_and_force_overwrite_behaviour() {
    let output_dir = unique_test_dir("e2e_transform_overwrite");
    fs::create_dir_all(&output_dir).expect("create output dir");

    let existing_file = output_dir.join("smith2020machine.md");
    fs::write(&existing_file, "sentinel output\n").expect("write sentinel output");

    let skipped = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
        ],
        Some("n\n"),
    );

    assert!(skipped.status.success());
    assert!(stderr_text(&skipped).contains("Warning: Skipped existing file:"));
    assert_eq!(
        fs::read_to_string(&existing_file).expect("read sentinel"),
        "sentinel output\n"
    );

    let overwritten = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--overwrite",
        ],
        None,
    );

    assert!(overwritten.status.success());
    let overwritten_text = fs::read_to_string(&existing_file).expect("read overwritten file");
    assert!(overwritten_text.contains("Machine Learning for Natural Language Processing"));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_file_name_strategy_001_generates_expected_file_names() {
    let slugify_dir = unique_test_dir("e2e_transform_slugify");
    let slugify = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            slugify_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
        ],
        None,
    );
    assert!(slugify.status.success());
    assert!(slugify_dir.join("smith2020machine.md").exists());

    let uuid8_dir = unique_test_dir("e2e_transform_uuid8");
    let uuid8 = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            uuid8_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
        ],
        None,
    );
    assert!(uuid8.status.success());

    for key in [
        "smith2020machine",
        "doe2019deep",
        "wang2021transformer",
        "brown2018attention",
        "alice2022blog",
        "carol2020thesis",
    ] {
        let expected = utils::generate_output_filename(key, FileNameStrategy::Uuid8, "md");
        assert!(uuid8_dir.join(expected).exists());
    }

    let _ = fs::remove_dir_all(&slugify_dir);
    let _ = fs::remove_dir_all(&uuid8_dir);
}

#[test]
fn e2e_transform_file_name_collision_001_disambiguates_colliding_slugified_keys() {
    let fixture_dir = unique_test_dir("e2e_transform_file_name_collision");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let input_path = fixture_dir.join("input_colliding_keys.bib");
    fs::write(
        &input_path,
        concat!(
            "@article{proj.a2026,\n",
            "  title = {Alpha},\n",
            "  author = {Doe, John},\n",
            "  year = {2026}\n",
            "}\n",
            "\n",
            "@article{proj:a2026,\n",
            "  title = {Beta},\n",
            "  author = {Smith, Jane},\n",
            "  year = {2026}\n",
            "}\n"
        ),
    )
    .expect("write colliding-keys input bib");

    let template_path = fixture_dir.join("template_title.md");
    fs::write(&template_path, "{{ title }}\n").expect("write title template");

    let output_dir = unique_test_dir("e2e_transform_file_name_collision_output");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            input_path.to_str().expect("input path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert!(
        stderr_text(&output).contains("Warning: Output filename collision"),
        "{}",
        stderr_text(&output)
    );

    let first =
        fs::read_to_string(output_dir.join("proj_a2026.md")).expect("read first colliding file");
    let second = fs::read_to_string(output_dir.join("proj_a2026-2.md"))
        .expect("read disambiguated second colliding file");
    assert!(first.contains("Alpha"));
    assert!(second.contains("Beta"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_file_name_collision_002_declined_overwrite_does_not_poison_tracking() {
    let fixture_dir = unique_test_dir("e2e_transform_file_name_collision_declined");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let input_path = fixture_dir.join("input_colliding_keys.bib");
    fs::write(
        &input_path,
        concat!(
            "@article{proj.a2026,\n",
            "  title = {Alpha},\n",
            "  author = {Doe, John},\n",
            "  year = {2026}\n",
            "}\n",
            "\n",
            "@article{proj:a2026,\n",
            "  title = {Beta},\n",
            "  author = {Smith, Jane},\n",
            "  year = {2026}\n",
            "}\n"
        ),
    )
    .expect("write colliding-keys input bib");

    let template_path = fixture_dir.join("template_title.md");
    fs::write(&template_path, "{{ title }}\n").expect("write title template");

    let output_dir = unique_test_dir("e2e_transform_file_name_collision_declined_output");
    fs::create_dir_all(&output_dir).expect("create output dir");
    let existing_file = output_dir.join("proj_a2026.md");
    fs::write(&existing_file, "sentinel output\n").expect("write sentinel output");

    // Decline the first entry's overwrite prompt, then accept the second's,
    // since both colliding keys target the same pre-existing file name.
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            input_path.to_str().expect("input path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
        ],
        Some("n\ny\n"),
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    assert!(stderr_text(&output).contains("Warning: Skipped existing file:"));
    assert!(
        !stderr_text(&output).contains("Warning: Output filename collision"),
        "declined overwrite must not be treated as a same-run collision: {}",
        stderr_text(&output)
    );
    assert!(!output_dir.join("proj_a2026-2.md").exists());

    let final_content = fs::read_to_string(&existing_file).expect("read final file content");
    assert!(final_content.contains("Beta"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_single_001_generates_single_output_file() {
    let output_dir = unique_test_dir("e2e_transform_single");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry_single.json")
                .to_str()
                .expect("template path"),
            "--single",
        ],
        None,
    );

    assert!(output.status.success());
    let output_file = output_dir.join("input_sample_template_entry_single.json");
    assert!(output_file.exists());

    let json = read_json_file(&output_file);
    assert_eq!(json["entries"].as_array().expect("entries array").len(), 6);

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_include_type_001_filters_entries_by_type() {
    let output_dir = unique_test_dir("e2e_transform_include_type");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include-type",
            "article",
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));

    let generated_files = fs::read_dir(&output_dir)
        .expect("list output dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();

    assert!(!generated_files.is_empty());
    assert!(generated_files.len() < 6);

    for file in generated_files {
        let content = fs::read_to_string(&file).expect("read output file");
        assert!(content.contains("**Type**: article"));
    }

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_exclude_type_001_filters_entries_by_type() {
    let output_dir = unique_test_dir("e2e_transform_exclude_type");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--exclude-type",
            "article",
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));

    let generated_files = fs::read_dir(&output_dir)
        .expect("list output dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();

    assert!(!generated_files.is_empty());
    assert!(generated_files.len() < 6);

    for file in generated_files {
        let content = fs::read_to_string(&file).expect("read output file");
        assert!(!content.contains("**Type**: article"));
    }

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_type_selection_001_rejects_conflicting_type_options() {
    let output_dir = unique_test_dir("e2e_transform_type_selection_conflict");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--include-type",
            "article",
            "--exclude-type",
            "book",
        ],
        None,
    );

    assert!(!output.status.success());
    assert!(
        stderr_text(&output)
            .contains("Cannot specify both --exclude-type and --include-type at the same time")
    );
}

#[test]
fn e2e_transform_verbose_001_switches_logging_style() {
    let verbose_dir = unique_test_dir("e2e_transform_verbose");
    let verbose = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            verbose_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--verbose",
            "--overwrite",
        ],
        None,
    );
    assert!(verbose.status.success());
    let verbose_stderr = stderr_text(&verbose);
    assert!(verbose_stderr.contains("Configuration:"));
    assert!(verbose_stderr.contains("Processing 6 entries"));
    assert!(verbose_stderr.contains("Summary:"));

    let quiet_dir = unique_test_dir("e2e_transform_quiet");
    let quiet = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            quiet_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--overwrite",
        ],
        None,
    );
    assert!(quiet.status.success());
    let quiet_stderr = stderr_text(&quiet);
    assert!(quiet_stderr.contains("Summary:"));
    assert!(!quiet_stderr.contains("Configuration:"));

    let _ = fs::remove_dir_all(&verbose_dir);
    let _ = fs::remove_dir_all(&quiet_dir);
}

#[test]
fn e2e_transform_latex_substitution_map_001_applies_custom_overrides() {
    let fixture_dir = unique_test_dir("e2e_transform_latex_substitution_map_fixture");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let template_path = fixture_dir.join("template_latex_substitution.md");
    fs::write(
        &template_path,
        "{{ latex_substitute(value=\"A \\\\textemdash B\") }}\n",
    )
    .expect("write latex substitute template");

    let custom_map_path = fixture_dir.join("custom_substitution_map.json");
    fs::write(&custom_map_path, "{\"\\\\textemdash\": \"--\"}\n")
        .expect("write custom substitution map");

    let default_output_dir = unique_test_dir("e2e_transform_latex_substitution_default");
    let default_output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            default_output_dir.to_str().expect("default output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include",
            "smith2020machine",
        ],
        None,
    );

    assert!(default_output.status.success());
    let default_rendered = fs::read_to_string(default_output_dir.join("smith2020machine.md"))
        .expect("read default substitution output");
    assert!(default_rendered.contains("A — B"));

    let custom_output_dir = unique_test_dir("e2e_transform_latex_substitution_custom");
    let custom_output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            custom_output_dir.to_str().expect("custom output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include",
            "smith2020machine",
            "--latex-substitution-map",
            custom_map_path.to_str().expect("custom map path"),
        ],
        None,
    );

    assert!(custom_output.status.success());
    let custom_rendered = fs::read_to_string(custom_output_dir.join("smith2020machine.md"))
        .expect("read custom substitution output");
    assert!(custom_rendered.contains("A -- B"));
    assert!(!custom_rendered.contains("A — B"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&default_output_dir);
    let _ = fs::remove_dir_all(&custom_output_dir);
}

#[test]
fn e2e_transform_latex_substitution_cascade_001_does_not_chain_replacement_output() {
    let fixture_dir = unique_test_dir("e2e_transform_latex_substitution_cascade");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let input_path = fixture_dir.join("input_latex_substitution_cascade.bib");
    fs::write(
        &input_path,
        concat!(
            "@article{cascade2026,\n",
            "  title = {TOKENLONG},\n",
            "  author = {Doe, John},\n",
            "  year = {2026}\n",
            "}\n"
        ),
    )
    .expect("write cascade input bib");

    let template_path = fixture_dir.join("template_latex_substitution_cascade.md");
    fs::write(&template_path, "{{ latex_substitute(value=title) }}\n")
        .expect("write latex substitute template");

    let custom_map_path = fixture_dir.join("custom_substitution_map.json");
    fs::write(
        &custom_map_path,
        "{\"TOKENLONG\": \"TOK\", \"TOK\": \"DONE\"}\n",
    )
    .expect("write custom substitution map");

    let output_dir = unique_test_dir("e2e_transform_latex_substitution_cascade_output");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            input_path.to_str().expect("input path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include",
            "cascade2026",
            "--latex-substitution-map",
            custom_map_path.to_str().expect("custom map path"),
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    let rendered =
        fs::read_to_string(output_dir.join("cascade2026.md")).expect("read rendered output");
    assert!(rendered.contains("TOK"));
    assert!(!rendered.contains("DONE"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_latex_substitution_math_mode_002_preserves_math_regions() {
    let fixture_dir = unique_test_dir("e2e_transform_latex_substitution_math_mode_fixture");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let input_path = fixture_dir.join("input_math_mode.bib");
    fs::write(
        &input_path,
        concat!(
            "@article{mathmode2026,\n",
            "  title = {outside TOKEN; $inline TOKEN$; $$display TOKEN$$; \\(paren TOKEN\\); \\[bracket TOKEN\\]},\n",
            "  author = {Doe, John},\n",
            "  year = {2026}\n",
            "}\n"
        ),
    )
    .expect("write math-mode input bib");

    let template_path = fixture_dir.join("template_latex_math_mode.md");
    fs::write(&template_path, "{{ latex_substitute(value=title) }}\n")
        .expect("write latex substitute template");

    let custom_map_path = fixture_dir.join("custom_substitution_map.json");
    fs::write(&custom_map_path, "{\"TOKEN\": \"CHANGED\"}\n")
        .expect("write custom substitution map");

    let output_dir = unique_test_dir("e2e_transform_latex_substitution_math_mode");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            input_path.to_str().expect("input path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include",
            "mathmode2026",
            "--latex-substitution-map",
            custom_map_path.to_str().expect("custom map path"),
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    let rendered =
        fs::read_to_string(output_dir.join("mathmode2026.md")).expect("read rendered output");
    assert!(rendered.contains("outside CHANGED;"));
    assert!(rendered.contains("$inline TOKEN$;"));
    assert!(rendered.contains("$$display TOKEN$$;"));
    assert!(rendered.contains("\\(paren TOKEN\\);"));
    assert!(rendered.contains("\\[bracket TOKEN\\]"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_latex_substitution_math_mode_003_preserves_real_default_tokens_in_math_regions() {
    let fixture_dir = unique_test_dir("e2e_transform_latex_substitution_math_mode_real_tokens");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let input_path = fixture_dir.join("input_math_mode_real_tokens.bib");
    fs::write(
        &input_path,
        concat!(
            "@article{realtokens2026,\n",
            "  title = {outside \\textemdash \\textasciitilde \\textasciicircum; $inline \\textemdash \\textasciitilde \\textasciicircum$; $$display \\textemdash \\textasciitilde \\textasciicircum$$; \\(paren \\textemdash \\textasciitilde \\textasciicircum\\); \\[bracket \\textemdash \\textasciitilde \\textasciicircum\\]},\n",
            "  author = {Doe, John},\n",
            "  year = {2026}\n",
            "}\n"
        ),
    )
    .expect("write math-mode input bib");

    let template_path = fixture_dir.join("template_latex_math_mode_real_tokens.md");
    fs::write(&template_path, "{{ latex_substitute(value=title) }}\n")
        .expect("write latex substitute template");

    let output_dir = unique_test_dir("e2e_transform_latex_substitution_math_mode_real_tokens");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            input_path.to_str().expect("input path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include",
            "realtokens2026",
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    let rendered =
        fs::read_to_string(output_dir.join("realtokens2026.md")).expect("read rendered output");
    assert!(rendered.contains("outside —~^;"));
    assert!(rendered.contains("$inline \\textemdash \\textasciitilde \\textasciicircum$;"));
    assert!(rendered.contains("$$display \\textemdash \\textasciitilde \\textasciicircum$$;"));
    assert!(rendered.contains("\\(paren \\textemdash \\textasciitilde \\textasciicircum\\);"));
    assert!(rendered.contains("\\[bracket \\textemdash \\textasciitilde \\textasciicircum\\]"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_latex_substitution_math_mode_004_treats_unclosed_double_dollar_as_plain_text() {
    let fixture_dir =
        unique_test_dir("e2e_transform_latex_substitution_math_mode_unclosed_double_dollar");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");

    let input_path = fixture_dir.join("input_math_mode_unclosed_double_dollar.bib");
    fs::write(
        &input_path,
        concat!(
            "@article{uncloseddoubledollar2026,\n",
            "  title = {$$unclosed TOKEN $real$ math TOKEN},\n",
            "  author = {Doe, John},\n",
            "  year = {2026}\n",
            "}\n"
        ),
    )
    .expect("write unclosed-double-dollar input bib");

    let template_path = fixture_dir.join("template_latex_math_mode_unclosed_double_dollar.md");
    fs::write(&template_path, "{{ latex_substitute(value=title) }}\n")
        .expect("write latex substitute template");

    let custom_map_path = fixture_dir.join("custom_substitution_map.json");
    fs::write(&custom_map_path, "{\"TOKEN\": \"CHANGED\"}\n")
        .expect("write custom substitution map");

    let output_dir =
        unique_test_dir("e2e_transform_latex_substitution_math_mode_unclosed_double_dollar");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            input_path.to_str().expect("input path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            template_path.to_str().expect("template path"),
            "--file-name-strategy",
            "slugify",
            "--include",
            "uncloseddoubledollar2026",
            "--latex-substitution-map",
            custom_map_path.to_str().expect("custom map path"),
        ],
        None,
    );

    assert!(output.status.success(), "{}", stderr_text(&output));
    let rendered = fs::read_to_string(output_dir.join("uncloseddoubledollar2026.md"))
        .expect("read rendered output");
    assert!(rendered.contains("$$unclosed CHANGED $real$ math CHANGED"));

    let _ = fs::remove_dir_all(&fixture_dir);
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_transform_errors_001_reports_invalid_input_and_template_failures() {
    let malformed_dir = unique_test_dir("e2e_transform_errors");
    fs::create_dir_all(&malformed_dir).expect("create malformed dir");
    let malformed_bib = malformed_dir.join("malformed.bib");
    fs::write(&malformed_bib, "this is not valid bibtex").expect("write malformed bib");

    let malformed = run_bibtera(
        &[
            "transform",
            "-i",
            malformed_bib.to_str().expect("malformed bib path"),
            "-o",
            malformed_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry.md")
                .to_str()
                .expect("template path"),
        ],
        None,
    );
    assert!(!malformed.status.success());
    assert!(stderr_text(&malformed).contains("Failed to parse BibTeX file"));

    let missing_template = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "-o",
            malformed_dir.to_str().expect("output dir"),
            "-t",
            malformed_dir
                .join("missing.md")
                .to_str()
                .expect("missing template path"),
        ],
        None,
    );
    assert!(!missing_template.status.success());
    assert!(stderr_text(&missing_template).contains("Template path does not exist"));

    let _ = fs::remove_dir_all(&malformed_dir);
}

#[test]
fn e2e_transform_large_dataset_001_renders_large_single_output() {
    let output_dir = unique_test_dir("e2e_transform_large_dataset");
    let output = run_bibtera(
        &[
            "transform",
            "-i",
            examples_dir()
                .join("input_iclr2025_1k.bib")
                .to_str()
                .expect("large bib path"),
            "-o",
            output_dir.to_str().expect("output dir"),
            "-t",
            examples_dir()
                .join("template_entry_single.json")
                .to_str()
                .expect("template path"),
            "--single",
        ],
        None,
    );

    assert!(output.status.success());
    let output_file = output_dir.join("input_iclr2025_1k_template_entry_single.json");
    assert!(output_file.exists());

    let json = read_json_file(&output_file);
    assert!(json["entries"].as_array().expect("entries array").len() > 500);

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn e2e_info_types_001_reports_default_supported_entry_types() {
    let output = run_bibtera(&["info"], None);
    assert!(output.status.success());

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    assert!(json.get("article").is_some());
    assert!(json["article"].get("author_parts").is_some());
}

#[test]
fn e2e_info_input_types_001_reports_types_present_in_input_file() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
        ],
        None,
    );
    assert!(output.status.success());

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    assert!(json.get("article").is_some());
    assert!(json.get("book").is_some());
    assert!(json.get("inproceedings").is_some());
    assert!(json.get("misc").is_some());
    assert!(json.get("phdthesis").is_some());
    assert!(json.get("mastersthesis").is_none());
}

#[test]
fn e2e_info_selection_001_reports_selected_entries() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "--include",
            "smith2020machine",
        ],
        None,
    );
    assert!(output.status.success());

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    let object = json.as_object().expect("top-level info object");
    assert_eq!(object.len(), 1);
    assert_eq!(
        json["smith2020machine"]["title"],
        "Machine Learning for Natural Language Processing"
    );
}

#[test]
fn e2e_info_include_type_001_reports_selected_entries_by_type() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "--include-type",
            "article",
        ],
        None,
    );
    assert!(output.status.success(), "{}", stderr_text(&output));

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    let object = json.as_object().expect("top-level info object");
    assert!(!object.is_empty());

    for entry in object.values() {
        assert_eq!(entry["entry_type"], "article");
    }
}

#[test]
fn e2e_info_exclude_type_001_reports_selected_entries_by_type() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "--exclude-type",
            "article",
        ],
        None,
    );
    assert!(output.status.success(), "{}", stderr_text(&output));

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    let object = json.as_object().expect("top-level info object");
    assert!(!object.is_empty());

    for entry in object.values() {
        assert_ne!(entry["entry_type"], "article");
    }
}

#[test]
fn e2e_info_type_selection_001_rejects_conflicting_type_options() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "--include-type",
            "article",
            "--exclude-type",
            "book",
        ],
        None,
    );

    assert!(!output.status.success());
    assert!(
        stderr_text(&output)
            .contains("Cannot specify both --exclude-type and --include-type at the same time")
    );
}

#[test]
fn e2e_info_field_map_accuracy_001_matches_template_context() {
    let default_output = run_bibtera(&["info"], None);
    assert!(default_output.status.success());

    let input_output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
        ],
        None,
    );
    assert!(input_output.status.success());

    for output in [&default_output, &input_output] {
        let json: Value = serde_json::from_str(&stdout_text(output)).expect("parse info json");
        for (entry_type, field_map) in json.as_object().expect("top-level info object") {
            let field_map = field_map.as_object().expect("entry type field map");

            for top_level_only in ["fields.author", "fields.title", "fields.year"] {
                assert!(
                    !field_map.contains_key(top_level_only),
                    "{} must not advertise {}",
                    entry_type,
                    top_level_only
                );
            }

            for top_level in ["authors", "author_parts", "title", "year"] {
                assert!(
                    field_map.contains_key(top_level),
                    "{} must advertise top-level key {}",
                    entry_type,
                    top_level
                );
            }
        }
    }

    // No entry in input_sample.bib has a month field, so the input-derived map
    // must not advertise fields.month for any entry type.
    let input_json: Value =
        serde_json::from_str(&stdout_text(&input_output)).expect("parse info json");
    for (entry_type, field_map) in input_json.as_object().expect("top-level info object") {
        assert!(
            !field_map
                .as_object()
                .expect("entry type field map")
                .contains_key("fields.month"),
            "{} must not advertise fields.month for a month-less input",
            entry_type
        );
    }
}

#[test]
fn e2e_info_empty_selection_001_reports_empty_map_with_warning() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_sample.bib")
                .to_str()
                .expect("sample bib path"),
            "--include",
            "nosuchkey",
        ],
        None,
    );
    assert!(output.status.success(), "{}", stderr_text(&output));

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    assert!(
        json.as_object().expect("top-level info object").is_empty(),
        "expected an empty map, got: {}",
        stdout_text(&output)
    );
    assert!(
        stderr_text(&output).contains("matched no entries"),
        "expected a no-match warning on stderr, got: {}",
        stderr_text(&output)
    );
}

#[test]
fn e2e_info_verbose_001_surfaces_error_cause_chain() {
    let malformed_dir = unique_test_dir("e2e_info_verbose");
    fs::create_dir_all(&malformed_dir).expect("create malformed dir");
    let malformed_bib = malformed_dir.join("malformed.bib");
    fs::write(&malformed_bib, "this is not valid bibtex").expect("write malformed bib");
    let malformed_path = malformed_bib.to_str().expect("malformed bib path");

    let quiet = run_bibtera(&["info", "-i", malformed_path], None);
    assert!(!quiet.status.success());
    assert!(stderr_text(&quiet).contains("Failed to parse BibTeX file"));
    assert!(
        !stderr_text(&quiet).contains("Caused by:"),
        "non-verbose info must not print the cause chain, got: {}",
        stderr_text(&quiet)
    );

    let verbose = run_bibtera(&["info", "-i", malformed_path, "--verbose"], None);
    assert!(!verbose.status.success());
    assert!(stderr_text(&verbose).contains("Failed to parse BibTeX file"));
    assert!(
        stderr_text(&verbose).contains("Caused by:"),
        "verbose info must print the cause chain, got: {}",
        stderr_text(&verbose)
    );
    assert!(
        stderr_text(&verbose).contains("Configuration:"),
        "verbose info must echo the resolved configuration, got: {}",
        stderr_text(&verbose)
    );

    let _ = fs::remove_dir_all(&malformed_dir);
}

#[test]
fn e2e_info_large_dataset_001_reports_representative_selected_entries() {
    let output = run_bibtera(
        &[
            "info",
            "-i",
            examples_dir()
                .join("input_iclr2025_1k.bib")
                .to_str()
                .expect("large bib path"),
            "--include",
            "DBLP:conf/iclr/0001000DFNC25,DBLP:conf/iclr/00010025,DBLP:conf/iclr/000100CLH025",
        ],
        None,
    );
    assert!(output.status.success());

    let json: Value = serde_json::from_str(&stdout_text(&output)).expect("parse info json");
    let object = json.as_object().expect("top-level info object");
    assert_eq!(object.len(), 3);
    assert!(json.get("DBLP:conf/iclr/0001000DFNC25").is_some());
    assert!(json.get("DBLP:conf/iclr/00010025").is_some());
    assert!(json.get("DBLP:conf/iclr/000100CLH025").is_some());
}
