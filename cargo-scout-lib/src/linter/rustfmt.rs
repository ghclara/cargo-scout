use crate::error::Error;
use crate::linter::{Lint, Linter, Location};
use serde::Deserialize;
use serde_json;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default)]
pub struct RustFmt {}

impl Linter for RustFmt {
    fn lints(&self, working_dir: PathBuf) -> Result<Vec<Lint>, Error> {
        println!(
            "[RustFmt] - checking format for directory {}",
            &working_dir.to_str().unwrap_or("<no directory>")
        );
        let rustfmt_output = Self::fmt(working_dir)?;
        lints(&rustfmt_output)
    }
}

impl RustFmt {
    fn command_parameters() -> Vec<&'static str> {
        vec!["+nightly", "fmt", "--", "--emit", "json"]
    }
    fn fmt(path: impl AsRef<Path>) -> Result<String, Error> {
        let fmt_output = Command::new("cargo")
            .current_dir(path)
            .args(Self::command_parameters())
            .output()
            .expect("failed to run cargo fmt");

        if fmt_output.status.success() {
            Ok(String::from_utf8(fmt_output.stdout)?)
        } else {
            Err(Error::Command(String::from_utf8(fmt_output.stderr)?))
        }
    }
}

#[derive(Deserialize, Debug)]
struct FmtLint {
    name: String,
    mismatches: Vec<FmtMismatch>,
}

#[derive(Deserialize, Debug)]
struct FmtMismatch {
    original_begin_line: u32,
    original_end_line: u32,
    original: String,
    expected: String,
}

fn lints(fmt_output: &str) -> Result<Vec<Lint>, Error> {
    let mut lints = Vec::new();
    let fmt_lints: Vec<FmtLint> = serde_json::from_str(fmt_output)?;
    for fmt_lint in fmt_lints {
        lints.append(
            &mut fmt_lint
                .mismatches
                .iter()
                .map(|missmatch| {
                    let path = fmt_lint.name.clone();
                    Lint {
                        message: display_missmatch(missmatch, &path),
                        location: Location {
                            path,
                            lines: [missmatch.original_begin_line, missmatch.original_end_line],
                        },
                    }
                })
                .collect::<Vec<Lint>>(),
        );
    }
    Ok(lints)
}

fn display_missmatch(missmatch: &FmtMismatch, path: &str) -> String {
    if missmatch.original_begin_line == missmatch.original_end_line {
        format!(
            "Diff in {} at line {}: \n-{}\n+{}\n",
            path, missmatch.original_begin_line, missmatch.original, missmatch.expected
        )
    } else {
        format!(
            "Diff in {} between lines {} and {}: \n{}\n{}\n",
            path,
            missmatch.original_begin_line,
            missmatch.original_end_line,
            missmatch
                .original
                .lines()
                .map(|line| format!("-{}", line))
                .collect::<Vec<String>>()
                .join("\n"),
            missmatch
                .expected
                .lines()
                .map(|line| format!("+{}", line))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}
