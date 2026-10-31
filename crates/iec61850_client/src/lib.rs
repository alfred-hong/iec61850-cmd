//! `iec61850_client` command dispatch.
//!
//! Guards that must not touch the network are decided here. Device commands
//! open one association, use the loaded model, and send MMS Conclude.

mod cli;
mod device;

use std::time::Duration;

use async_trait::async_trait;
use clap::Parser;
use iec61850_cmd::iec61850::report::Report;
use iec61850_cmd::iec61850::Iec61850ClientError;
use iec61850_cmd::mms::ReportCallback;

use crate::cli::{Cli, Command, ControlWhat, GetWhat, GooseWhat, SetWhat, SubscribeWhat, SvWhat};

/// Printed when `control operate` has no `--yes`.
pub const NEED_YES_CONTROL: &str = "存在控制，但是该控制未生效，需要添加确认标识 --yes\n";
/// Printed when `--test=false` or `--simulation=false` has no `--live`.
pub const NEED_LIVE: &str = "关闭测试位或仿真位需要添加 --live\n";
/// Printed when a device command has neither `--host` nor `IEC61850_HOST`.
pub const NEED_HOST: &str = "需要 --host 或环境变量 IEC61850_HOST\n";
/// Printed when `set object` is asked to write `FC=CO`.
pub const REJECT_CO: &str = "action=write\n拒绝 FC=CO\n";
/// Printed for an unknown `--format`.
pub const BAD_FORMAT: &str = "未知的 --format\n";
/// Printed for an unknown `--type`.
pub const BAD_TYPE: &str = "未知的 --type\n";
/// Printed when `--timeout` or `--connect-timeout` is not a duration.
pub const BAD_TIMEOUT: &str = "无法解析超时\n";
/// Printed when a CSV read is given `--format`.
pub const CSV_NO_FORMAT: &str = "读值固定 CSV，不用 --format\n";
/// Printed when `--entry-id` is not 16 hex digits.
pub const BAD_ENTRY: &str = "EntryID 须为 16 个十六进制字符\n";
/// Printed for a parsed command that is not wired to the device yet.
pub const NOT_YET: &str = "尚未实现\n";
/// Printed when `control operate --yes` reads ctlModel 0.
pub const STATUS_ONLY: &str = "status-only，对象不可控\n";
/// Printed when an object reference is not `LD/LN.DO.DA`.
pub const BAD_REF: &str = "引用格式须为 LD/LN.DO.DA\n";
/// Printed when `--ln` is not `LD/LN`.
pub const BAD_LN_REF: &str = "引用格式须为 LD/LN\n";
/// Printed when `--dataset` is not `LD/LN.数据集名`.
pub const BAD_DS_REF: &str = "引用格式须为 LD/LN.数据集名\n";
/// Printed when `--value` does not match `--type`.
pub const BAD_VALUE: &str = "无法解析 --value\n";
/// Printed when `get object` has neither `--object` nor `--csv`.
pub const NEED_OBJECT: &str = "需要 --object 和 --fc，或 --csv\n";
/// Printed when `get object` sets both `--object` and `--csv`.
pub const OBJECT_CSV_MUTEX: &str = "--object 与 --csv 互斥\n";
/// Printed when `get object --object` has no `--fc`.
pub const NEED_FC: &str = "需要 --fc\n";
/// Printed when a read CSV names a live column.
pub const CSV_LIVE_COLUMNS: &str = "CSV 不能包含 value、type、quality 或 time\n";
/// Printed when `--csv` cannot be opened.
pub const CSV_OPEN: &str = "无法打开 --csv\n";
/// Printed when a read CSV has no `reference` and `fc` columns, or a short row.
pub const CSV_SHAPE: &str = "CSV 须带 reference 和 fc 列，且每行列数与表头一致\n";
/// Printed when `set object --verify` reads back a different value.
pub const VERIFY_MISMATCH: &str = "读回不一致\n";

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

pub(crate) struct IgnoreReport;

#[async_trait]
impl ReportCallback for IgnoreReport {
	async fn on_report(&self, _report: Report) {}
}

/// Parse `args` (without argv0) and either print a guard or talk to the device.
pub fn run<I, S>(args: I) -> Outcome
where
	I: IntoIterator<Item = S>,
	S: Into<std::ffi::OsString> + Clone,
{
	let mut argv = vec!["iec61850_client".into()];
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
	match &cli.command {
		Command::Version => ok(format!("iec61850_client {}\n", env!("CARGO_PKG_VERSION"))),
		Command::Get { .. } if cli.format.is_some() => fail(2, CSV_NO_FORMAT),
		Command::Control {
			what: ControlWhat::Operate { yes: false, object, .. },
		} => operate_without_yes(&cli, object),
		Command::Goose {
			what: GooseWhat::Publish { test, live, .. },
		} => publish_guard(!*test, *live),
		Command::Sv {
			what: SvWhat::Publish {
				simulation, live, ..
			},
		} => publish_guard(!*simulation, *live),
		Command::Set {
			what: SetWhat::Object { fc, .. },
		} if fc.eq_ignore_ascii_case("CO") => fail(2, REJECT_CO),
		Command::Subscribe {
			what: SubscribeWhat::Report { entry_id, .. },
		} => {
			if let Some(entry) = entry_id {
				if !is_entry_id(entry) {
					return fail(2, BAD_ENTRY);
				}
			}
			need_host_then_later(&cli)
		}
		Command::Get {
			what: cli::GetWhat::Journal { .. },
		}
		| Command::Control {
			what: ControlWhat::Inspect { .. },
		} => need_host_then_later(&cli),
		Command::Goose {
			what: GooseWhat::List { .. },
		} => fail(1, NOT_YET),
		Command::List { .. }
		| Command::Tree
		| Command::Find { .. }
		| Command::Get { .. }
		| Command::Set { .. }
		| Command::Control {
			what: ControlWhat::Operate { yes: true, .. },
		} => device::run(&cli),
	}
}

/// `control operate` without `--yes` does not operate. It prints the notice,
/// then reads the status value: `LD/LN.DO` becomes `LD/LN.DO.stVal` with `FC=ST`.
fn operate_without_yes(cli: &Cli, object: &str) -> Outcome {
	let mut stderr = String::from(NEED_YES_CONTROL);
	if cli.format.is_some() {
		stderr.push_str(CSV_NO_FORMAT);
		return Outcome { code: 2, stdout: String::new(), stderr };
	}
	let Ok(reference) = status_reference(object) else {
		stderr.push_str(BAD_REF);
		return Outcome { code: 2, stdout: String::new(), stderr };
	};
	if cli.host.as_ref().is_none_or(|host| host.is_empty()) {
		stderr.push_str(NEED_HOST);
		return Outcome { code: 2, stdout: String::new(), stderr };
	}
	let read = Cli {
		host: cli.host.clone(),
		port: cli.port,
		timeout: cli.timeout.clone(),
		connect_timeout: cli.connect_timeout.clone(),
		ied_name: cli.ied_name.clone(),
		format: None,
		command: Command::Get {
			what: GetWhat::Object {
				object: Some(reference),
				fc: Some("ST".to_owned()),
				csv: None,
			},
		},
	};
	let mut outcome = device::run(&read);
	outcome.stderr.insert_str(0, &stderr);
	outcome
}

/// A controllable data object `LD/LN.DO` has its status at `stVal`.
/// A reference that already names an attribute is read as written.
fn status_reference(object: &str) -> Result<String, ()> {
	let (logical_device, rest) = object.split_once('/').ok_or(())?;
	if logical_device.is_empty() || rest.is_empty() || rest.contains('/') {
		return Err(());
	}
	let mut parts = rest.split('.');
	let logical_node = parts.next().filter(|part| !part.is_empty()).ok_or(())?;
	let data_object = parts.next().filter(|part| !part.is_empty()).ok_or(())?;
	if parts.next().is_none() {
		Ok(format!("{logical_device}/{logical_node}.{data_object}.stVal"))
	} else {
		Ok(object.to_owned())
	}
}

fn publish_guard(turning_off: bool, live: bool) -> Outcome {
	if turning_off && !live {
		return fail(2, NEED_LIVE);
	}
	fail(1, NOT_YET)
}

fn need_host_then_later(cli: &Cli) -> Outcome {
	if cli.host.as_ref().is_none_or(|host| host.is_empty()) {
		fail(2, NEED_HOST)
	} else {
		fail(1, NOT_YET)
	}
}

/// Connection and request timeouts are exit 4. Other association failures are exit 3.
fn association_exit(err: &Iec61850ClientError) -> i32 {
	let mut current: Option<&dyn std::error::Error> = Some(err);
	while let Some(item) = current {
		let text = item.to_string().to_ascii_lowercase();
		if text.contains("timed out") || text.contains("connection timeout") {
			return 4;
		}
		current = item.source();
	}
	3
}

fn error_chain(err: &Iec61850ClientError) -> String {
	let mut text = err.to_string();
	let mut source = std::error::Error::source(err);
	while let Some(inner) = source {
		text.push_str(": ");
		text.push_str(&inner.to_string());
		source = std::error::Error::source(inner);
	}
	text
}

fn is_entry_id(text: &str) -> bool {
	text.len() == 16 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn normalize_type(text: &str) -> Option<&'static str> {
	match text.to_ascii_lowercase().as_str() {
		"bool" | "boolean" => Some("bool"),
		"int" | "integer" => Some("int"),
		"uint" | "unsigned" => Some("uint"),
		"float" | "double" => Some("float"),
		"enum" => Some("enum"),
		"string" => Some("string"),
		_ => None,
	}
}

fn parse_duration(text: &str) -> Result<Duration, ()> {
	if let Some(ms) = text.strip_suffix("ms") {
		return Ok(Duration::from_millis(ms.parse().map_err(|_| ())?));
	}
	if let Some(seconds) = text.strip_suffix('s') {
		return Ok(Duration::from_secs(seconds.parse().map_err(|_| ())?));
	}
	Ok(Duration::from_secs(text.parse().map_err(|_| ())?))
}

fn ok(stdout: String) -> Outcome {
	Outcome {
		code: 0,
		stdout,
		stderr: String::new(),
	}
}

fn fail(code: i32, stderr: &str) -> Outcome {
	Outcome {
		code,
		stdout: String::new(),
		stderr: stderr.to_owned(),
	}
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
			format!("iec61850_client {}\n", env!("CARGO_PKG_VERSION"))
		);
	}

	#[test]
	fn help_is_success() {
		let outcome = run(["--help"]);
		assert_eq!(outcome.code, 0);
		assert!(outcome.stdout.contains("version"));
	}

	#[test]
	fn operate_with_yes_needs_a_host() {
		let outcome = run([
			"control", "operate", "--object", "LD/LN.DO", "--value", "1", "--type", "bool", "--yes",
		]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, NEED_HOST);
	}

	#[test]
	fn operate_without_yes_reads_status_and_needs_a_host() {
		let outcome = run([
			"control", "operate", "--object", "LD/LN.DO", "--value", "1", "--type", "bool",
		]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stdout, "");
		assert_eq!(outcome.stderr, format!("{NEED_YES_CONTROL}{NEED_HOST}"));
	}

	#[test]
	fn control_object_status_is_stval() {
		assert_eq!(status_reference("LD/LN.DO").as_deref(), Ok("LD/LN.DO.stVal"));
		assert_eq!(status_reference("LD/LN.DO.stVal").as_deref(), Ok("LD/LN.DO.stVal"));
		assert!(status_reference("not-a-ref").is_err());
	}

	#[test]
	fn operate_without_yes_still_tries_to_read() {
		let outcome = run([
			"--host",
			"127.0.0.1",
			"--port",
			"1",
			"--connect-timeout",
			"1s",
			"control",
			"operate",
			"--object",
			"LD/LN.DO",
			"--value",
			"1",
			"--type",
			"bool",
		]);
		assert_eq!(outcome.code, 3);
		assert!(outcome.stderr.starts_with(NEED_YES_CONTROL));
		assert!(outcome.stderr.contains("action=get object\n"));
	}

	#[test]
	fn publish_does_not_require_yes() {
		let outcome = run([
			"goose",
			"publish",
			"--iface",
			"eth0",
			"--scl",
			"station.scd",
			"--gocb",
			"gcb01",
		]);
		assert_eq!(outcome.code, 1);
		assert_eq!(outcome.stderr, NOT_YET);
	}

	#[test]
	fn clearing_the_test_bit_requires_live() {
		let outcome = run([
			"goose",
			"publish",
			"--iface",
			"eth0",
			"--scl",
			"station.scd",
			"--gocb",
			"gcb01",
			"--test=false",
		]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, NEED_LIVE);
	}

	#[test]
	fn set_object_rejects_fc_co() {
		let outcome = run([
			"set",
			"object",
			"--object",
			"LD/LN.DO.ctlVal",
			"--fc",
			"CO",
			"--value",
			"1",
			"--type",
			"bool",
		]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, REJECT_CO);
	}

	#[test]
	fn unknown_format_is_exit_2() {
		let outcome = run(["--format", "yaml", "version"]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, BAD_FORMAT);
	}

	#[test]
	fn list_lds_without_host_is_exit_2() {
		let outcome = run(["list", "lds"]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, NEED_HOST);
	}

	#[test]
	fn entry_id_must_be_sixteen_hex_digits() {
		let outcome = run([
			"--host",
			"192.0.2.10",
			"subscribe",
			"report",
			"--ld",
			"LD0",
			"--ln",
			"LLN0",
			"--report",
			"rcb01",
			"--entry-id",
			"abc",
		]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, BAD_ENTRY);
	}

	#[test]
	fn bad_object_reference_does_not_connect() {
		let outcome = run(["get", "object", "--object", "nope", "--fc", "ST"]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, BAD_REF);
	}

	#[test]
	fn bad_value_does_not_connect() {
		let outcome = run([
			"set", "object", "--object", "LD/LN.DO.stVal", "--fc", "ST", "--value", "maybe",
			"--type", "bool",
		]);
		assert_eq!(outcome.code, 2);
		assert_eq!(outcome.stderr, BAD_VALUE);
	}

	#[test]
	fn refused_connection_is_exit_3() {
		let outcome = run([
			"--host",
			"127.0.0.1",
			"--port",
			"1",
			"--connect-timeout",
			"1s",
			"list",
			"lds",
		]);
		assert_eq!(outcome.code, 3, "{}", outcome.stderr);
		assert!(outcome.stderr.starts_with("action=list lds\n"));
	}
}
