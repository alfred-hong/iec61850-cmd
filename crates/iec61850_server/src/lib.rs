//! `iec61850_server` command dispatch.
//!
//! Step 1 keeps an empty shell: `version` runs, `start --scl` is parsed and
//! then returns exit 1 with `尚未实现`. Listening and SCL load wait for
//! PLAN_server.md Step 5.

mod cli;

use clap::Parser;

use crate::cli::{Cli, Command};

/// Printed for a parsed `start` that is not wired yet.
pub const NOT_YET: &str = "尚未实现\n";
/// Printed for an unknown `--format`.
pub const BAD_FORMAT: &str = "未知的 --format\n";

/// What `run` decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
	/// Process exit code.
	pub code: i32,
	/// stdout, including a trailing newline when there is text.
	pub stdout: String,
	/// stderr, including a trailing newline when there is text.
	pub stderr: String,
}

/// Parse `args` (without argv0) and either print a guard or stop.
pub fn run<I, S>(args: I) -> Outcome
where
	I: IntoIterator<Item = S>,
	S: Into<std::ffi::OsString> + Clone,
{
	let mut argv = vec!["iec61850_server".into()];
	argv.extend(args.into_iter().map(Into::into));
	match Cli::try_parse_from(argv) {
		Ok(cli) => dispatch(cli),
		Err(err) => Outcome {
			code: i32::from(err.exit_code()),
			stdout: if err.kind() == clap::error::ErrorKind::DisplayHelp {
				err.to_string()
			} else {
				String::new()
			},
			stderr: if err.kind() == clap::error::ErrorKind::DisplayHelp {
				String::new()
			} else {
				err.to_string()
			},
		},
	}
}

/// Run and write the outcome to the process stdout and stderr.
pub fn run_to_stdio() -> i32 {
	let outcome = run(std::env::args_os().skip(1));
	print!("{}", outcome.stdout);
	eprint!("{}", outcome.stderr);
	outcome.code
}

fn dispatch(cli: Cli) -> Outcome {
	if let Some(format) = &cli.format {
		if !matches!(format.as_str(), "text" | "json" | "jsonl") {
			return fail(2, BAD_FORMAT);
		}
	}
	match cli.command {
		Command::Version => ok(format!("iec61850_server {}\n", env!("CARGO_PKG_VERSION"))),
		Command::Start { .. } => fail(1, NOT_YET),
	}
}

fn ok(stdout: String) -> Outcome {
	Outcome { code: 0, stdout, stderr: String::new() }
}

fn fail(code: i32, stderr: &str) -> Outcome {
	Outcome { code, stdout: String::new(), stderr: stderr.to_owned() }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn version_prints_the_package_version() {
		let outcome = run(["version"]);
		assert_eq!(outcome.code, 0);
		assert_eq!(
			outcome.stdout,
			format!("iec61850_server {}\n", env!("CARGO_PKG_VERSION"))
		);
	}

	#[test]
	fn help_is_success() {
		let outcome = run(["--help"]);
		assert_eq!(outcome.code, 0);
		assert!(outcome.stdout.contains("version"));
		assert!(outcome.stdout.contains("start"));
	}

	#[test]
	fn start_without_scl_is_exit_2() {
		let outcome = run(["start"]);
		assert_eq!(outcome.code, 2);
	}

	#[test]
	fn start_with_scl_is_not_yet() {
		let outcome = run(["start", "--scl", "station.scd"]);
		assert_eq!(outcome.code, 1);
		assert_eq!(outcome.stderr, NOT_YET);
	}

	#[test]
	fn unknown_format_is_exit_2() {
		let outcome = run(["--format", "yaml", "version"]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, BAD_FORMAT);
	}
}
