//! Device commands that the library can already perform.
//!
//! One association loads the model. Reads and writes use that association.
//! The command then sends MMS Conclude.

use std::sync::atomic::{AtomicU32, Ordering};

use iec61850_cmd::iec61850::data::Iec61850Data;
use iec61850_cmd::iec61850::model::{IedModel, LogicalNode, Node};
use iec61850_cmd::iec61850::Iec61850ClientError;
use iec61850_cmd::mms::asn1::mms::asn1::ServiceErrorErrorClass;
use iec61850_cmd::mms::client::{MmsClientError, ServiceFailure};
use iec61850_cmd::mms::ClientConfig;
use iec61850_cmd::path::{object_to_mms, parse_cli_object};
use iec61850_cmd::Iec61850Client;

use crate::cli::{Cli, Command, ControlWhat, FindWhat, GetWhat, ListWhat, SetWhat};
use crate::{
	association_exit, error_chain, fail, ok, parse_duration, IgnoreReport, Outcome, BAD_DS_REF,
	BAD_LN_REF, BAD_REF, BAD_TIMEOUT, BAD_TYPE, BAD_VALUE, CSV_LIVE_COLUMNS, CSV_OPEN, CSV_SHAPE,
	NEED_FC, NEED_HOST, NEED_OBJECT, NOT_YET, OBJECT_CSV_MUTEX, REJECT_CO, STATUS_ONLY,
	VERIFY_MISMATCH,
};

const LIVE: [&str; 4] = ["value", "type", "quality", "time"];

struct InputRow {
	reference: String,
	fc: String,
	prefix: Vec<String>,
}

struct ReadPlan {
	header: Vec<String>,
	rows: Vec<InputRow>,
}

pub(crate) fn run(cli: &Cli) -> Outcome {
	if let Some(problem) = validate(cli) {
		return problem;
	}
	let Some(host) = cli.host.as_ref().filter(|host| !host.is_empty()) else {
		return fail(2, NEED_HOST);
	};
	let mut config = ClientConfig {
		address: host.clone(),
		port: cli.port,
		..ClientConfig::default()
	};
	match (parse_duration(&cli.timeout), parse_duration(&cli.connect_timeout)) {
		(Ok(timeout), Ok(connect)) => {
			config.connection.request_timeout = Some(timeout);
			config.connection.connect_timeout = connect;
		}
		_ => return fail(2, BAD_TIMEOUT),
	}
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(err) => return fail(1, &format!("{err}\n")),
	};
	let mut stderr = match &cli.command {
		Command::Control {
			what: ControlWhat::Operate { object, value, .. },
		} => format!("action=operate {object} {value}\n"),
		command => format!("action={}\n", action_of(command)),
	};
	let client = match runtime.block_on(Iec61850Client::new(config, Box::new(IgnoreReport))) {
		Ok(client) => client,
		Err(err) => {
			let code = association_exit(&err);
			stderr.push_str(&error_chain(&err));
			stderr.push('\n');
			return Outcome { code, stdout: String::new(), stderr };
		}
	};
	runtime.block_on(async move {
		let mut outcome = execute(&client, cli).await;
		outcome.stderr.insert_str(0, &stderr);
		outcome.stderr.push_str("action=release\n");
		if let Err(err) = client.close().await {
			if outcome.code == 0 {
				outcome.code = association_exit(&err);
			}
			outcome.stderr.push_str(&error_chain(&err));
			outcome.stderr.push('\n');
		}
		outcome
	})
}

fn validate(cli: &Cli) -> Option<Outcome> {
	match &cli.command {
		Command::Get { what: GetWhat::Object { object, fc, csv } } => {
			match (object.as_ref(), csv.as_ref()) {
				(Some(_), Some(_)) => return Some(fail(2, OBJECT_CSV_MUTEX)),
				(None, None) => return Some(fail(2, NEED_OBJECT)),
				(Some(reference), None) => {
					let Some(fc) = fc.as_ref() else {
						return Some(fail(2, NEED_FC));
					};
					if parse_cli_object(reference, fc).is_err() {
						return Some(fail(2, BAD_REF));
					}
				}
				(None, Some(path)) => {
					let text = match std::fs::read_to_string(path) {
						Ok(text) => text,
						Err(_) => return Some(fail(2, CSV_OPEN)),
					};
					if let Err(message) = parse_csv(&text) {
						return Some(fail(2, message));
					}
				}
			}
		}
		Command::Get { what: GetWhat::Ln { ln, .. } } => {
			if split_ld_ln(ln).is_none() {
				return Some(fail(2, BAD_LN_REF));
			}
		}
		Command::Get { what: GetWhat::Ds { dataset } } => {
			if split_dataset(dataset).is_none() {
				return Some(fail(2, BAD_DS_REF));
			}
		}
		Command::Set { what: SetWhat::Object { object, fc, value, r#type, .. } } => {
			if fc.eq_ignore_ascii_case("CO") {
				return Some(fail(2, REJECT_CO));
			}
			let Some(kind) = crate::normalize_type(r#type) else {
				return Some(fail(2, BAD_TYPE));
			};
			if parse_cli_object(object, fc).is_err() {
				return Some(fail(2, BAD_REF));
			}
			if parse_value(kind, value).is_none() {
				return Some(fail(2, BAD_VALUE));
			}
		}
		Command::Control {
			what: ControlWhat::Operate { object, value, r#type, .. },
		} => {
			if control_target(object).is_none() {
				return Some(fail(2, BAD_REF));
			}
			let Some(kind) = crate::normalize_type(r#type) else {
				return Some(fail(2, BAD_TYPE));
			};
			if parse_value(kind, value).is_none() {
				return Some(fail(2, BAD_VALUE));
			}
		}
		_ => {}
	}
	None
}

async fn execute(client: &Iec61850Client, cli: &Cli) -> Outcome {
	let format = cli.format.as_deref();
	let model = client.model();
	match &cli.command {
		Command::List { what: ListWhat::Lds } => ok(emit_names(format, &sorted_names(model))),
		Command::List { what: ListWhat::Lns { ld } } => {
			if has_ld(model, ld) {
				ok(emit_names(format, &ln_names(model, ld)))
			} else {
				missing()
			}
		}
		Command::List { what: ListWhat::Dos { ld, ln } } => {
			list_children(model, format, ld, ln, false, None)
		}
		Command::List { what: ListWhat::Das { ld, ln, data_object } } => {
			list_children(model, format, ld, ln, true, Some(data_object))
		}
		Command::List { what: ListWhat::Dss { ld, ln } } => match find_ln(model, ld, Some(ln)) {
			Some(node) => {
				let mut names = node
					.datasets
					.values()
					.map(|dataset| dataset.path.replace('$', "."))
					.collect::<Vec<_>>();
				names.sort();
				ok(emit_names(format, &names))
			}
			None => missing(),
		},
		Command::List { what: ListWhat::Reports { ld, ln } } => {
			match find_ln(model, ld, Some(ln)) {
				Some(node) => {
					let mut names = node
						.reports
						.values()
						.map(|report| format!("{ld}/{}", report.name.replace('$', ".")))
						.collect::<Vec<_>>();
					names.sort();
					ok(emit_names(format, &names))
				}
				None => missing(),
			}
		}
		Command::List { what: ListWhat::Journals { ld } } => list_journals(client, model, format, ld.as_deref()).await,
		Command::Tree => ok(render_tree(model, format)),
		Command::Find { what: FindWhat::Path { ln, path } } => {
			let names = find_paths(model, ln, path);
			if names.is_empty() {
				missing()
			} else {
				ok(emit_names(format, &names))
			}
		}
		Command::Get { what: GetWhat::Object { object, fc, csv } } => {
			let plan = if let Some(path) = csv {
				let text = std::fs::read_to_string(path).unwrap_or_default();
				match parse_csv(&text) {
					Ok(plan) => plan,
					Err(message) => return fail(2, message),
				}
			} else {
				let reference = object.clone().unwrap_or_default();
				let fc = fc.clone().unwrap_or_default();
				ReadPlan {
					header: ["reference", "fc", "value", "type", "quality", "time"]
						.into_iter()
						.map(str::to_owned)
						.collect(),
					rows: vec![InputRow {
						prefix: vec![reference.clone(), fc.clone()],
						reference,
						fc,
					}],
				}
			};
			read_plan(client, model, plan).await
		}
		Command::Get { what: GetWhat::Ln { ln, fc } } => read_ln(client, model, ln, fc.as_deref()).await,
		Command::Get { what: GetWhat::Ds { dataset } } => read_ds(client, model, dataset).await,
		Command::Set { what: SetWhat::Object { object, fc, value, r#type, verify } } => {
			write_object(client, model, object, fc, value, r#type, *verify).await
		}
		Command::Control {
			what: ControlWhat::Operate { object, value, r#type, .. },
		} => operate_direct(client, model, object, value, r#type).await,
		_ => fail(1, NOT_YET),
	}
}

fn list_children(
	model: &IedModel,
	format: Option<&str>,
	ld: &str,
	ln: &str,
	attributes: bool,
	data_object: Option<&String>,
) -> Outcome {
	let Some(node) = find_ln(model, ld, Some(ln)) else {
		return missing();
	};
	let mut lines = Vec::new();
	for fc in &node.nodes {
		for child in fc.object_children() {
			let do_name = child.name();
			if attributes {
				let wanted = data_object.map(String::as_str).unwrap_or("");
				if wanted != do_name && wanted != format!("{}.{}", fc.name(), do_name) {
					continue;
				}
				for attr in child.object_children() {
					let ty = attr.attribute_type().unwrap_or("");
					// Same leaf name can exist under more than one FC. Keep the FC.
					lines.push(format!("{} {} {ty}", fc.name(), attr.name()));
				}
			} else {
				// ACSI data-object names, matching iec61850ctl `list dos`.
				lines.push(do_name.to_owned());
			}
		}
	}
	lines.sort();
	lines.dedup();
	if attributes && lines.is_empty() {
		return missing();
	}
	lines.sort();
	ok(emit_names(format, &lines))
}

async fn list_journals(
	client: &Iec61850Client,
	model: &IedModel,
	format: Option<&str>,
	only: Option<&str>,
) -> Outcome {
	let devices = if let Some(ld) = only {
		if !has_ld(model, ld) {
			return missing();
		}
		vec![ld.to_owned()]
	} else {
		sorted_names(model)
	};
	let mut lines = Vec::new();
	for ld in devices {
		match client.get_journal_names(&ld).await {
			Ok(names) => {
				let mut names = names;
				names.sort();
				lines.extend(names.into_iter().map(|name| format!("{ld}/{name}")));
			}
			Err(err) => return service_failure(err),
		}
	}
	ok(emit_names(format, &lines))
}

async fn read_plan(client: &Iec61850Client, model: &IedModel, plan: ReadPlan) -> Outcome {
	let mut rendered = Vec::new();
	let mut codes = Vec::new();
	let mut stderr = String::new();
	for row in plan.rows {
		let (cells, code, detail) = read_row(client, model, &row).await;
		if let Some(detail) = detail {
			stderr.push_str(&detail);
			stderr.push('\n');
		}
		rendered.push(cells);
		codes.push(code);
	}
	let mut stdout = plan.header.join(",");
	stdout.push('\n');
	for cells in rendered {
		stdout.push_str(&cells.iter().map(|cell| csv_cell(cell)).collect::<Vec<_>>().join(","));
		stdout.push('\n');
	}
	Outcome { code: worst(&codes), stdout, stderr }
}

async fn read_row(
	client: &Iec61850Client,
	model: &IedModel,
	row: &InputRow,
) -> (Vec<String>, i32, Option<String>) {
	let mut prefix = row.prefix.clone();
	let fail_row = |code: i32, typ: &str, detail: Option<String>| {
		let mut cells = prefix.clone();
		cells.extend([String::new(), typ.to_owned(), "-".to_owned(), "-".to_owned()]);
		(cells, code, detail)
	};
	let Ok(object) = parse_cli_object(&row.reference, &row.fc) else {
		return fail_row(2, "ERROR", Some(BAD_REF.trim_end().to_owned()));
	};
	if !has_ld(model, &object.logical_device) {
		return fail_row(5, "NOT_ON_DEVICE", None);
	}
	let item = object_to_mms(&object);
	let values = match client.read_data(&format!("{}/{}", object.logical_device, item)).await {
		Ok(values) => values,
		Err(err) => {
			let code = service_code(&err);
			return fail_row(code, type_for(code), Some(error_chain(&err)));
		}
	};
	let Some(value) = values.into_iter().next_back() else {
		return fail_row(1, "ERROR", None);
	};
	let (quality, time) = sibling_qt(client, &object).await;
	prefix.push(value.csv_value());
	prefix.push(value.mms_type_name().to_owned());
	prefix.push(quality);
	prefix.push(time);
	(prefix, 0, None)
}

async fn sibling_qt(
	client: &Iec61850Client,
	object: &iec61850_cmd::path::CliObject,
) -> (String, String) {
	let data = &object.data_path;
	let wanted = data.ends_with(".stVal") || data.ends_with("Op.general");
	if !wanted {
		return ("-".to_owned(), "-".to_owned());
	}
	let Some((parent, _)) = data.rsplit_once('.') else {
		return ("-".to_owned(), "-".to_owned());
	};
	let quality = read_sibling(client, object, &format!("{parent}.q")).await;
	let time = read_sibling(client, object, &format!("{parent}.t")).await;
	(quality, time)
}

async fn read_sibling(
	client: &Iec61850Client,
	object: &iec61850_cmd::path::CliObject,
	data_path: &str,
) -> String {
	let mut sibling = object.clone();
	sibling.data_path = data_path.to_owned();
	let item = object_to_mms(&sibling);
	let Ok(mut values) = client.read_data(&format!("{}/{}", sibling.logical_device, item)).await
	else {
		return "-".to_owned();
	};
	match values.pop() {
		Some(Iec61850Data::BitString(bits)) if bits.bytes.iter().all(|byte| *byte == 0) => {
			"good:0000".to_owned()
		}
		Some(Iec61850Data::BitString(bits)) => {
			bits.bytes.iter().map(|byte| format!("{byte:02X}")).collect()
		}
		Some(value) => value.csv_value(),
		None => "-".to_owned(),
	}
}

async fn read_ln(
	client: &Iec61850Client,
	model: &IedModel,
	ln: &str,
	fc: Option<&str>,
) -> Outcome {
	let Some((ld, ln_name)) = split_ld_ln(ln) else {
		return fail(2, BAD_LN_REF);
	};
	if !has_ld(model, ld) {
		return csv_one(ln, fc.unwrap_or(""), "", "NOT_ON_DEVICE", 5);
	}
	let Some(node) = find_ln(model, ld, Some(ln_name)) else {
		return csv_one(ln, fc.unwrap_or(""), "", "NOT_FOUND", 5);
	};
	let mut leaves = Vec::new();
	for fc_node in &node.nodes {
		if fc.is_some_and(|wanted| !fc_node.name().eq_ignore_ascii_case(wanted)) {
			continue;
		}
		collect_leaves(fc_node, &mut leaves);
	}
	let rows = leaves
		.iter()
		.filter_map(|leaf| leaf_row(leaf))
		.map(|(reference, fc, _item)| InputRow {
			prefix: vec![reference.clone(), fc.clone()],
			reference,
			fc,
		})
		.collect();
	read_plan(
		client,
		model,
		ReadPlan {
			header: ["reference", "fc", "value", "type", "quality", "time"]
				.into_iter()
				.map(str::to_owned)
				.collect(),
			rows,
		},
	)
	.await
}

async fn read_ds(client: &Iec61850Client, model: &IedModel, dataset: &str) -> Outcome {
	let Some((ld, ln_name, name)) = split_dataset(dataset) else {
		return fail(2, BAD_DS_REF);
	};
	if !has_ld(model, ld) {
		return csv_one(dataset, "", "", "NOT_ON_DEVICE", 5);
	}
	let Some(node) = find_ln(model, ld, Some(ln_name)) else {
		return csv_one(dataset, "", "", "NOT_FOUND", 5);
	};
	let key = format!("{ln_name}${name}");
	let Some(found) = node.datasets.get(&key) else {
		return csv_one(dataset, "", "", "NOT_FOUND", 5);
	};
	let path = format!("{ld}/{key}");
	let values = match client.read_dataset(&path.as_str().into()).await {
		Ok(values) => values,
		Err(err) => {
			let code = service_code(&err);
			let mut outcome = csv_one(dataset, "", "", type_for(code), code);
			outcome.stderr = error_chain(&err);
			outcome.stderr.push('\n');
			return outcome;
		}
	};
	let mut rows = Vec::new();
	let mut codes = Vec::new();
	for (index, value) in values.iter().enumerate() {
		let reference = found.entries.get(index).map(String::as_str).unwrap_or(dataset);
		let (shown, fc) = member_ref(reference);
		let (quality, time) = match parse_cli_object(&shown, &fc) {
			Ok(object) => sibling_qt(client, &object).await,
			Err(_) => ("-".to_owned(), "-".to_owned()),
		};
		rows.push(vec![
			shown,
			fc,
			value.csv_value(),
			value.mms_type_name().to_owned(),
			quality,
			time,
		]);
		codes.push(0);
	}
	if found.entries.len() != values.len() {
		codes.push(1);
	}
	let mut stdout = "reference,fc,value,type,quality,time\n".to_owned();
	for cells in rows {
		stdout.push_str(&cells.iter().map(|cell| csv_cell(cell)).collect::<Vec<_>>().join(","));
		stdout.push('\n');
	}
	Outcome { code: worst(&codes), stdout, stderr: String::new() }
}

async fn write_object(
	client: &Iec61850Client,
	model: &IedModel,
	object: &str,
	fc: &str,
	value: &str,
	kind: &str,
	verify: bool,
) -> Outcome {
	let Ok(parsed) = parse_cli_object(object, fc) else {
		return fail(2, BAD_REF);
	};
	let Some(kind) = crate::normalize_type(kind) else {
		return fail(2, BAD_TYPE);
	};
	let Some(data) = parse_value(kind, value) else {
		return fail(2, BAD_VALUE);
	};
	// The command wrapper already printed `action=write` before associate.
	let mut stderr = String::new();
	if !has_ld(model, &parsed.logical_device) {
		stderr.push_str("NOT_ON_DEVICE\n");
		return Outcome { code: 5, stdout: String::new(), stderr };
	}
	let item = object_to_mms(&parsed);
	let path = format!("{}/{}", parsed.logical_device, item);
	if let Err(err) = client.set_data_value(&path.as_str().into(), data.clone()).await {
		let code = service_code(&err);
		stderr.push_str(&error_chain(&err));
		stderr.push('\n');
		return Outcome { code, stdout: String::new(), stderr };
	}
	if verify {
		stderr.push_str("action=read\n");
		match client.read_data(&path).await {
			Ok(mut values) => {
				let same = values.pop().is_some_and(|got| got.csv_value() == data.csv_value());
				if !same {
					stderr.push_str(VERIFY_MISMATCH);
					return Outcome { code: 7, stdout: String::new(), stderr };
				}
			}
			Err(err) => {
				let code = service_code(&err);
				stderr.push_str(&error_chain(&err));
				stderr.push('\n');
				return Outcome { code, stdout: String::new(), stderr };
			}
		}
	}
	Outcome { code: 0, stdout: String::new(), stderr }
}

fn leaf_row(node: &Node) -> Option<(String, String, String)> {
	let path = node.path();
	let (ld, rest) = path.split_once('/')?;
	let mut parts = rest.split('$');
	let ln = parts.next()?;
	let fc = parts.next()?;
	let data = parts.collect::<Vec<_>>();
	if data.is_empty() {
		return None;
	}
	let reference = format!("{ld}/{ln}.{}", data.join("."));
	let item = rest.to_owned();
	Some((reference, fc.to_owned(), item))
}

fn collect_leaves<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
	if node.attribute_type().is_some() {
		out.push(node);
		return;
	}
	for child in node.object_children() {
		collect_leaves(child, out);
	}
}

fn find_paths(model: &IedModel, ln_query: &str, data_path: &str) -> Vec<String> {
	let mut names = Vec::new();
	for ld in &model.logical_devices {
		for ln in &ld.logical_nodes {
			if !ln.name.contains(ln_query) {
				continue;
			}
			let mut leaves = Vec::new();
			for fc in &ln.nodes {
				collect_leaves(fc, &mut leaves);
			}
			for leaf in leaves {
				let Some((reference, fc, _)) = leaf_row(leaf) else {
					continue;
				};
				let after_ln = reference.split_once('/').and_then(|(_, rest)| rest.split_once('.'));
				let dotted = after_ln.map(|(_, data)| data).unwrap_or("");
				let with_fc = format!("{fc}.{dotted}");
				if dotted == data_path || with_fc == data_path {
					names.push(format!("{reference} {fc}"));
				}
			}
		}
	}
	names.sort();
	names
}

fn render_tree(model: &IedModel, format: Option<&str>) -> String {
	if matches!(format, Some("json" | "jsonl")) {
		return format!("{model}\n");
	}
	let mut lines = Vec::new();
	for ld in &model.logical_devices {
		lines.push(ld.name.clone());
		for ln in &ld.logical_nodes {
			lines.push(format!("  {}", ln.name));
			for fc in &ln.nodes {
				write_node(fc, 4, &mut lines);
			}
		}
	}
	emit_names(None, &lines)
}

fn write_node(node: &Node, indent: usize, lines: &mut Vec<String>) {
	let pad = " ".repeat(indent);
	match node.attribute_type() {
		Some(ty) => lines.push(format!("{pad}{} {ty}", node.name())),
		None => {
			lines.push(format!("{pad}{}", node.name()));
			for child in node.object_children() {
				write_node(child, indent + 2, lines);
			}
		}
	}
}

fn member_ref(reference: &str) -> (String, String) {
	let Some((ld, rest)) = reference.split_once('/') else {
		return (reference.to_owned(), String::new());
	};
	let mut parts = rest.split('$');
	let Some(ln) = parts.next() else {
		return (reference.to_owned(), String::new());
	};
	let Some(fc) = parts.next() else {
		return (format!("{ld}/{ln}"), String::new());
	};
	let data = parts.collect::<Vec<_>>();
	if data.is_empty() {
		return (format!("{ld}/{ln}"), fc.to_owned());
	}
	(format!("{ld}/{ln}.{}", data.join(".")), fc.to_owned())
}

/// `LD/LN.DO`, or the CSV leaf `LD/LN.DO.Oper.ctlVal`.
fn control_target(object: &str) -> Option<(String, String, String)> {
	let (logical_device, rest) = object.split_once('/')?;
	let rest = rest.strip_suffix(".Oper.ctlVal").unwrap_or(rest);
	let (logical_node, data) = rest.split_once('.')?;
	if logical_device.is_empty()
		|| logical_node.is_empty()
		|| data.is_empty()
		|| logical_node.contains('.')
		|| rest.contains('/')
	{
		return None;
	}
	Some((logical_device.to_owned(), logical_node.to_owned(), data.to_owned()))
}

fn next_ctl_num() -> u32 {
	static NEXT: AtomicU32 = AtomicU32::new(0);
	(NEXT.fetch_add(1, Ordering::Relaxed) % 255) + 1
}

/// Direct-with-normal-security: one MMS write of `{LN}$CO${DO}$Oper`.
/// ctlModel 0 is rejected. Other models are not wired yet.
async fn operate_direct(
	client: &Iec61850Client,
	model: &IedModel,
	object: &str,
	value: &str,
	kind: &str,
) -> Outcome {
	let Some((logical_device, logical_node, data)) = control_target(object) else {
		return fail(2, BAD_REF);
	};
	let Some(kind) = crate::normalize_type(kind) else {
		return fail(2, BAD_TYPE);
	};
	let Some(ctl_val) = parse_value(kind, value) else {
		return fail(2, BAD_VALUE);
	};
	if !has_ld(model, &logical_device) {
		return Outcome { code: 5, stdout: String::new(), stderr: "NOT_ON_DEVICE\n".to_owned() };
	}
	let mms_data = data.replace('.', "$");
	let model_path = format!("{logical_device}/{logical_node}$CF${mms_data}$ctlModel");
	let ctl_model = match client.read_data(&model_path).await {
		Ok(mut values) => values.pop(),
		Err(err) => {
			return Outcome {
				code: service_code(&err),
				stdout: String::new(),
				stderr: format!("{}\n", error_chain(&err)),
			};
		}
	};
	let code = match ctl_model {
		Some(Iec61850Data::Integer(code)) => code,
		Some(Iec61850Data::Unsigned(code)) => i32::try_from(code).unwrap_or(-1),
		_ => return fail(6, STATUS_ONLY),
	};
	if code == 0 {
		return fail(6, STATUS_ONLY);
	}
	if code != 1 {
		return fail(1, NOT_YET);
	}
	let oper = Iec61850Data::direct_operate(ctl_val, next_ctl_num());
	let path = format!("{logical_device}/{logical_node}$CO${mms_data}$Oper");
	if let Err(err) = client.set_data_value(&path.as_str().into(), oper).await {
		return Outcome {
			code: service_code(&err),
			stdout: String::new(),
			stderr: format!("{}\n", error_chain(&err)),
		};
	}
	ok(String::new())
}

fn csv_one(reference: &str, fc: &str, value: &str, typ: &str, code: i32) -> Outcome {
	let stdout = format!(
		"reference,fc,value,type,quality,time\n{},{},{},{},-,-\n",
		csv_cell(reference),
		csv_cell(fc),
		csv_cell(value),
		csv_cell(typ)
	);
	Outcome { code, stdout, stderr: String::new() }
}

fn parse_csv(text: &str) -> Result<ReadPlan, &'static str> {
	let table = parse_rows(text).map_err(|_| CSV_SHAPE)?;
	if table.is_empty() {
		return Err(CSV_SHAPE);
	}
	let header = &table[0];
	if header.iter().any(|name| LIVE.contains(&name.as_str())) {
		return Err(CSV_LIVE_COLUMNS);
	}
	let Some(reference_at) = header.iter().position(|name| name == "reference") else {
		return Err(CSV_SHAPE);
	};
	let Some(fc_at) = header.iter().position(|name| name == "fc") else {
		return Err(CSV_SHAPE);
	};
	let mut rows = Vec::new();
	for raw in table.iter().skip(1) {
		if raw.len() != header.len() {
			return Err(CSV_SHAPE);
		}
		if raw.iter().all(String::is_empty) {
			continue;
		}
		rows.push(InputRow {
			reference: raw[reference_at].clone(),
			fc: raw[fc_at].clone(),
			prefix: raw.clone(),
		});
	}
	let mut header = header.clone();
	header.extend(LIVE.iter().map(|name| (*name).to_owned()));
	Ok(ReadPlan { header, rows })
}

fn parse_rows(text: &str) -> Result<Vec<Vec<String>>, ()> {
	let mut rows = Vec::new();
	let mut row = Vec::new();
	let mut cell = String::new();
	let mut chars = text.chars().peekable();
	let mut quoted = false;
	while let Some(ch) = chars.next() {
		if quoted {
			if ch == '"' {
				if chars.peek() == Some(&'"') {
					chars.next();
					cell.push('"');
				} else {
					quoted = false;
				}
			} else {
				cell.push(ch);
			}
		} else if ch == '"' && cell.is_empty() {
			quoted = true;
		} else if ch == ',' {
			row.push(std::mem::take(&mut cell));
		} else if ch == '\n' {
			row.push(std::mem::take(&mut cell));
			if row.iter().any(|item| !item.is_empty()) {
				rows.push(std::mem::take(&mut row));
			} else {
				row.clear();
			}
		} else if ch != '\r' {
			cell.push(ch);
		}
	}
	if quoted {
		return Err(());
	}
	if !cell.is_empty() || !row.is_empty() {
		row.push(cell);
		rows.push(row);
	}
	Ok(rows)
}

fn parse_value(kind: &str, value: &str) -> Option<Iec61850Data> {
	match kind {
		"bool" => match value.to_ascii_lowercase().as_str() {
			"1" | "true" => Some(Iec61850Data::Bool(true)),
			"0" | "false" => Some(Iec61850Data::Bool(false)),
			_ => None,
		},
		"int" | "enum" => value.parse::<i32>().ok().map(Iec61850Data::Integer),
		"uint" => value.parse::<u32>().ok().map(Iec61850Data::Unsigned),
		"float" => value.parse::<f32>().ok().map(Iec61850Data::FloatingPoint),
		"string" => Some(Iec61850Data::String(value.to_owned())),
		_ => None,
	}
}

fn split_ld_ln(text: &str) -> Option<(&str, &str)> {
	let (ld, ln) = text.split_once('/')?;
	if ld.is_empty() || ln.is_empty() || ln.contains('/') || ln.contains('.') {
		None
	} else {
		Some((ld, ln))
	}
}

fn split_dataset(text: &str) -> Option<(&str, &str, &str)> {
	let (ld, rest) = text.split_once('/')?;
	let (ln, name) = rest.split_once('.')?;
	if ld.is_empty() || ln.is_empty() || name.is_empty() || rest.contains('/') {
		None
	} else {
		Some((ld, ln, name))
	}
}

fn has_ld(model: &IedModel, ld: &str) -> bool {
	model.logical_devices.iter().any(|device| device.name == ld)
}

fn find_ln<'a>(model: &'a IedModel, ld: &str, ln: Option<&str>) -> Option<&'a LogicalNode> {
	let device = model.logical_devices.iter().find(|device| device.name == ld)?;
	let ln = ln?;
	device.logical_nodes.iter().find(|node| node.name == ln)
}

fn ln_names(model: &IedModel, ld: &str) -> Vec<String> {
	model
		.logical_devices
		.iter()
		.find(|device| device.name == ld)
		.map(|device| {
			let mut names = device.logical_nodes.iter().map(|node| node.name.clone()).collect::<Vec<_>>();
			names.sort();
			names
		})
		.unwrap_or_default()
}

fn sorted_names(model: &IedModel) -> Vec<String> {
	let mut names = model.logical_devices.iter().map(|device| device.name.clone()).collect::<Vec<_>>();
	names.sort();
	names
}

fn missing() -> Outcome {
	Outcome { code: 5, stdout: String::new(), stderr: String::new() }
}

fn service_failure(err: Iec61850ClientError) -> Outcome {
	let code = service_code(&err);
	fail(code, &format!("{}\n", error_chain(&err)))
}

fn service_code(err: &Iec61850ClientError) -> i32 {
	if let Iec61850ClientError::Client { source, .. } = err {
		match source {
			MmsClientError::DataAccessError { error, .. } => {
				let code = error.to_string().parse().unwrap_or(-1);
				return exit_for_mms_data_access(code);
			}
			MmsClientError::ServiceFailed { failure, .. } => match failure {
				ServiceFailure::ConfirmedError { service_error, .. } => {
					if let ServiceErrorErrorClass::access(code) = &service_error.error_class {
						let code = code.to_string().parse().unwrap_or(-1);
						return exit_for_mms_access_class(code);
					}
					// Any other Confirmed-Error is a rejection. iec61850ctl
					// maps those, including a journal list the peer will not
					// serve, to exit 6. Access-class code 2 stays exit 5.
					return 6;
				}
				ServiceFailure::Rejected { .. } => return 6,
				ServiceFailure::ConnectionClosed { .. } => return 3,
				ServiceFailure::PduTooLarge { .. } => {}
			},
			_ => {}
		}
	}
	let mut current = Some(err as &dyn std::error::Error);
	let mut access: Option<i32> = None;
	let mut timeout = false;
	while let Some(item) = current {
		let text = item.to_string();
		let lower = text.to_ascii_lowercase();
		if lower.contains("timed out") || lower.contains("connection timeout") {
			timeout = true;
		}
		if let Some(code) = data_access_code(&lower) {
			access = Some(exit_for_mms_data_access(code));
		}
		current = item.source();
	}
	if timeout {
		4
	} else if let Some(code) = access {
		code
	} else {
		1
	}
}

/// ISO 9506-2 DataAccessError. `10` is object-non-existent.
/// iec61850ctl maps that code to exit 5 and every other data-access code to exit 6.
fn data_access_code(lower: &str) -> Option<i64> {
	let rest = lower.split("data access error:").nth(1)?.trim();
	rest.split_whitespace().next()?.parse().ok()
}

fn exit_for_mms_data_access(code: i64) -> i32 {
	if code == 10 { 5 } else { 6 }
}

/// ISO 9506-2 access-class service error. Code 2 is object-non-existent.
fn exit_for_mms_access_class(code: i64) -> i32 {
	if code == 2 { 5 } else { 6 }
}

fn type_for(code: i32) -> &'static str {
	match code {
		4 => "TIMEOUT",
		5 => "NOT_FOUND",
		6 => "REJECTED",
		8 => "UNSUPPORTED",
		_ => "ERROR",
	}
}

fn worst(codes: &[i32]) -> i32 {
	const RANK: [i32; 7] = [8, 6, 5, 4, 3, 2, 1];
	RANK.into_iter().find(|rank| codes.contains(rank)).unwrap_or(0)
}

fn csv_cell(text: &str) -> String {
	if text.contains([',', '"', '\n', '\r']) {
		format!("\"{}\"", text.replace('"', "\"\""))
	} else {
		text.to_owned()
	}
}

fn emit_names(format: Option<&str>, names: &[String]) -> String {
	match format {
		Some("json") => {
			let body = names.iter().map(|name| json_string(name)).collect::<Vec<_>>().join(",");
			format!("[{body}]\n")
		}
		Some("jsonl") => {
			let mut out = String::new();
			for name in names {
				out.push_str("{\"name\":");
				out.push_str(&json_string(name));
				out.push_str("}\n");
			}
			out
		}
		_ => {
			let mut out = names.join("\n");
			if !out.is_empty() {
				out.push('\n');
			}
			out
		}
	}
}

fn json_string(text: &str) -> String {
	let mut out = String::from("\"");
	for ch in text.chars() {
		match ch {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			other => out.push(other),
		}
	}
	out.push('"');
	out
}

fn action_of(command: &Command) -> &'static str {
	match command {
		Command::List { what: ListWhat::Lds } => "list lds",
		Command::List { what: ListWhat::Lns { .. } } => "list lns",
		Command::List { what: ListWhat::Dos { .. } } => "list dos",
		Command::List { what: ListWhat::Das { .. } } => "list das",
		Command::List { what: ListWhat::Dss { .. } } => "list dss",
		Command::List { what: ListWhat::Reports { .. } } => "list reports",
		Command::List { what: ListWhat::Journals { .. } } => "list journals",
		Command::Tree => "tree",
		Command::Find { .. } => "find path",
		Command::Get { what: GetWhat::Object { .. } } => "get object",
		Command::Get { what: GetWhat::Ln { .. } } => "get ln",
		Command::Get { what: GetWhat::Ds { .. } } => "get ds",
		Command::Get { what: GetWhat::Journal { .. } } => "get journal",
		Command::Set { .. } => "write",
		_ => "mms",
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use iec61850_cmd::iec61850::model::{LogicalDevice, LogicalNode};

	#[test]
	fn control_target_accepts_the_object_and_the_csv_leaf() {
		assert_eq!(
			control_target("HV_3_AH_04CTRL/Q0GAPC10.SPCSO"),
			Some((
				"HV_3_AH_04CTRL".to_owned(),
				"Q0GAPC10".to_owned(),
				"SPCSO".to_owned(),
			))
		);
		assert_eq!(
			control_target("HV_3_AH_04CTRL/Q0GAPC11.SPCSO.Oper.ctlVal"),
			Some((
				"HV_3_AH_04CTRL".to_owned(),
				"Q0GAPC11".to_owned(),
				"SPCSO".to_owned(),
			))
		);
		assert!(control_target("not-a-ref").is_none());
	}

	fn sample() -> IedModel {
		let st = Node::object(
			"ST",
			"LD/LN$ST",
			vec![Node::object(
				"SPCSO1",
				"LD/LN$ST$SPCSO1",
				vec![Node::attribute("stVal", "LD/LN$ST$SPCSO1$stVal", "bool")],
			)],
		);
		let mut ln = LogicalNode::new("LN".to_owned(), "LD");
		ln.nodes.push(st);
		let mut ld = LogicalDevice::new("LD".to_owned());
		ld.logical_nodes.push(ln);
		IedModel { logical_devices: vec![ld] }
	}

	#[test]
	fn dos_are_object_names() {
		let model = sample();
		let outcome = list_children(&model, None, "LD", "LN", false, None);
		assert_eq!(outcome.code, 0);
		assert_eq!(outcome.stdout, "SPCSO1\n");
	}

	#[test]
	fn das_keep_fc_and_type() {
		let model = sample();
		let name = "SPCSO1".to_owned();
		let outcome = list_children(&model, None, "LD", "LN", true, Some(&name));
		assert_eq!(outcome.stdout, "ST stVal bool\n");
	}

	#[test]
	fn object_nonexistent_matches_iec61850ctl_exit() {
		// ISO 9506-2 DataAccessError object-non-existent = 10 → exit 5.
		// object-access-denied = 3 → exit 6. Access-class service error 2 is
		// the same not-found class iec61850ctl uses.
		assert_eq!(exit_for_mms_data_access(10), 5);
		assert_eq!(exit_for_mms_data_access(3), 6);
		assert_eq!(exit_for_mms_access_class(2), 5);
		assert_eq!(exit_for_mms_access_class(0), 6);
		assert_eq!(data_access_code("data access error: 10"), Some(10));
	}

	#[test]
	fn find_matches_the_full_data_name() {
		let names = find_paths(&sample(), "L", "SPCSO1.stVal");
		assert_eq!(names, vec!["LD/LN.SPCSO1.stVal ST".to_owned()]);
	}

	#[test]
	fn csv_rejects_live_columns() {
		assert!(matches!(
			parse_csv("reference,fc,value\nLD/LN.DO.stVal,ST,1\n"),
			Err(message) if message == CSV_LIVE_COLUMNS
		));
	}

	#[test]
	fn bool_value_accepts_words_and_digits() {
		assert_eq!(parse_value("bool", "true"), Some(Iec61850Data::Bool(true)));
		assert_eq!(parse_value("bool", "0"), Some(Iec61850Data::Bool(false)));
		assert!(parse_value("bool", "maybe").is_none());
	}
}
