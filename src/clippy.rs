use serde::Deserialize;
use std::io::{self, Write};
use std::process::Command;

#[derive(Deserialize, PartialEq, Debug)]
pub struct LintCode {
    pub code: String,
    pub explanation: String,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct LintSpan {
    pub file_name: String,
    /// The line where the lint should be reported
    ///
    /// GitHub provides a line_start and a line_end.
    /// We should use the line_start in case of multi-line lints.
    /// (Why?)
    pub line_start: usize,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Lint {
    /// The lint message
    ///
    /// Example:
    ///
    /// unused variable: `count`
    pub package_id: String,
    pub src_path: Option<String>,
    pub message: Option<Message>,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Message {
    pub rendered: String,
    pub spans: Vec<Span>,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Span {
    pub file_name: String,
    pub line_start: i32,
    pub line_end: i32,
}

pub struct Linter {
    verbose: bool,
    no_default_features: bool,
    all_features: bool,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            verbose: false,
            no_default_features: false,
            all_features: false,
        }
    }

    pub fn set_verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
        self
    }

    pub fn set_no_default_features(&mut self, no_default_features: bool) -> &mut Self {
        self.no_default_features = no_default_features;
        self
    }

    pub fn set_all_features(&mut self, all_features: bool) -> &mut Self {
        self.all_features = all_features;
        self
    }

    pub fn get_lints(&self) -> Result<Vec<Lint>, crate::error::Error> {
        self.get_lints_for_project(std::fs::canonicalize(".")?)
    }

    pub fn can_run_in_workspace(&self) -> bool {
        !self.no_default_features
    }

    pub fn get_lints_for_members(
        &self,
        members: Vec<String>,
    ) -> Result<Vec<Lint>, crate::error::Error> {
        let current_dir = std::fs::canonicalize(".")?;
        let mut lints = Vec::new();
        for m in members {
            println!(
                "Running clippy on workspace member {:?}",
                current_dir.join(&m)
            );
            lints.extend(self.get_lints_for_project(current_dir.join(m))?);
        }
        Ok(lints)
    }

    fn get_lints_for_project(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<Lint>, crate::error::Error> {
        self.clippy(path).map(|output| lints(&output))
    }

    fn get_command_parameters(&self) -> Vec<&str> {
        let mut params = vec!["clippy", "--message-format", "json"];
        if self.verbose {
            params.push("--verbose");
        }
        if self.no_default_features {
            params.push("--no-default-features");
        }
        if self.all_features {
            params.push("--all-features");
        }
        params.append(&mut vec!["--", "-W", "clippy::pedantic"]);
        params
    }

    fn get_envs(&self) -> Vec<(&str, &str)> {
        let mut envs = vec![];
        if self.verbose {
            envs.push(("RUST_BACKTRACE", "full"));
        }
        envs
    }

    fn clippy(&self, path: impl AsRef<std::path::Path>) -> Result<String, crate::error::Error> {
        let clippy_pedantic_output = Command::new("cargo")
            .current_dir(path)
            .args(self.get_command_parameters())
            .envs(self.get_envs())
            .output()
            .expect("failed to run clippy pedantic");
        if self.verbose {
            println!(
                "{}",
                String::from_utf8(clippy_pedantic_output.stdout.clone())?
            );
        }
        if clippy_pedantic_output.status.success() {
            Ok(String::from_utf8(clippy_pedantic_output.stdout)?)
        } else if self.verbose {
            println!("Clippy run failed");
            println!("cleaning and building with full backtrace");
            let _ = Command::new("cargo")
                .args(&["clean"])
                .envs(self.get_envs())
                .output()
                .expect("failed to start cargo clean");
            let build = Command::new("cargo")
                .args(&["build"])
                .envs(self.get_envs())
                .output()
                .expect("failed to start cargo build");
            if build.status.success() {
                Err(String::from_utf8(build.stdout)?.into())
            } else {
                io::stdout().write_all(&build.stdout)?;
                Err(String::from_utf8(build.stderr)?.into())
            }
        } else {
            Err(String::from_utf8(clippy_pedantic_output.stderr)?.into())
        }
    }
}

pub fn lints(clippy_output: &str) -> Vec<Lint> {
    clippy_output
        .lines()
        .filter(|l| l.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|lint: &Lint| {
            if let Some(m) = &lint.message {
                !m.spans.is_empty()
            } else {
                false
            }
        })
        .collect::<Vec<Lint>>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_set_verbose() {
        use crate::clippy::Linter;

        let mut linter = Linter::new();
        assert_eq!(false, linter.verbose);

        let l2 = linter.set_verbose(true);
        assert_eq!(true, l2.verbose);

        let l3 = l2.set_verbose(false);
        assert_eq!(false, l3.verbose);
    }
    #[test]
    fn test_get_envs() {
        use crate::clippy::Linter;

        let mut linter = Linter::new();
        let mut expected_envs = vec![];
        assert_eq!(expected_envs, linter.get_envs());

        let verbose_linter = linter.set_verbose(true);
        expected_envs.push(("RUST_BACKTRACE", "full"));
        assert_eq!(expected_envs, verbose_linter.get_envs());
    }
    #[test]
    fn test_get_command_parameters() {
        use crate::clippy::Linter;

        let mut linter = Linter::new();
        let expected_command_parameters = vec![
            "clippy",
            "--message-format",
            "json",
            "--",
            "-W",
            "clippy::pedantic",
        ];

        assert_eq!(expected_command_parameters, linter.get_command_parameters());

        let verbose_linter = linter.set_verbose(true);
        let verbose_expected_command_parameters = vec![
            "clippy",
            "--message-format",
            "json",
            "--verbose",
            "--",
            "-W",
            "clippy::pedantic",
        ];
        assert_eq!(
            verbose_expected_command_parameters,
            verbose_linter.get_command_parameters()
        );

        let no_default_features_linter = linter.set_verbose(false).set_no_default_features(true);
        let no_default_features_command_parameters = vec![
            "clippy",
            "--message-format",
            "json",
            "--no-default-features",
            "--",
            "-W",
            "clippy::pedantic",
        ];
        assert_eq!(
            no_default_features_command_parameters,
            no_default_features_linter.get_command_parameters()
        );

        let all_features_linter = linter
            .set_verbose(false)
            .set_no_default_features(false)
            .set_all_features(true);
        let all_features_command_parameters = vec![
            "clippy",
            "--message-format",
            "json",
            "--all-features",
            "--",
            "-W",
            "clippy::pedantic",
        ];
        assert_eq!(
            all_features_command_parameters,
            all_features_linter.get_command_parameters()
        );
    }
    #[test]
    fn test_lints() {
        use crate::clippy::{lints, Lint, Message, Span};
        let expected_lints = vec![Lint {
            package_id: "cargo-scout".to_string(),
            src_path: Some("test/foo/bar.rs".to_string()),
            message: Some(Message {
                rendered: "this is a test lint".to_string(),
                spans: vec![Span {
                    file_name: "test/foo/baz.rs".to_string(),
                    line_start: 10,
                    line_end: 12,
                }],
            }),
        }];

        let clippy_output = r#"{"package_id": "cargo-scout","src_path": "test/foo/bar.rs","message": { "rendered": "this is a test lint","spans": [{"file_name": "test/foo/baz.rs","line_start": 10,"line_end": 12}]}}"#;

        assert_eq!(expected_lints, lints(clippy_output));
    }
}
