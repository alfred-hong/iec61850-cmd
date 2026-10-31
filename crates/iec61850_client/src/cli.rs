//! Command line shape from PLAN_client.md.

use clap::{Parser, Subcommand};

/// IEC 61850 client.
#[derive(Debug, Parser)]
#[command(name = "iec61850_client", about = "IEC 61850 client")]
pub struct Cli {
	/// Device address. Overrides `IEC61850_HOST`.
	#[arg(long, global = true, env = "IEC61850_HOST")]
	pub host: Option<String>,

	/// TCP port. Overrides `IEC61850_PORT`. Default 102.
	#[arg(long, global = true, env = "IEC61850_PORT", default_value_t = 102)]
	pub port: u16,

	/// Confirmed-request timeout. Default 10s.
	#[arg(long, global = true, default_value = "10s")]
	pub timeout: String,

	/// TCP connect timeout. Default 10s.
	#[arg(long, global = true, default_value = "10s")]
	pub connect_timeout: String,

	/// IED name. Not sent in MMS Initiate. Overrides `IEC61850_IED_NAME`.
	#[arg(long, global = true, env = "IEC61850_IED_NAME")]
	pub ied_name: Option<String>,

	/// `text`, `json`, or `jsonl`. Reads stay CSV and reject this flag.
	#[arg(long, global = true)]
	pub format: Option<String>,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
	/// List logical devices, nodes, data, data sets, reports, or journals.
	List {
		#[command(subcommand)]
		what: ListWhat,
	},
	/// Walk the model.
	Tree,
	/// Find a path.
	Find {
		#[command(subcommand)]
		what: FindWhat,
	},
	/// Read a value, a logical node, a data set, or a journal.
	Get {
		#[command(subcommand)]
		what: GetWhat,
	},
	/// Write a scalar attribute. `FC=CO` is rejected.
	Set {
		#[command(subcommand)]
		what: SetWhat,
	},
	/// Subscribe to a report.
	Subscribe {
		#[command(subcommand)]
		what: SubscribeWhat,
	},
	/// Inspect or operate a control.
	Control {
		#[command(subcommand)]
		what: ControlWhat,
	},
	/// GOOSE.
	Goose {
		#[command(subcommand)]
		what: GooseWhat,
	},
	/// Sampled values.
	Sv {
		#[command(subcommand)]
		what: SvWhat,
	},
	/// Print the client version.
	Version,
}

#[derive(Debug, Subcommand)]
pub enum ListWhat {
	Lds,
	Lns {
		#[arg(long)]
		ld: String,
	},
	Dos {
		#[arg(long)]
		ld: String,
		#[arg(long)]
		ln: String,
	},
	Das {
		#[arg(long)]
		ld: String,
		#[arg(long)]
		ln: String,
		#[arg(long = "do")]
		data_object: String,
	},
	Dss {
		#[arg(long)]
		ld: String,
		#[arg(long)]
		ln: String,
	},
	Reports {
		#[arg(long)]
		ld: String,
		#[arg(long)]
		ln: String,
	},
	Journals {
		#[arg(long)]
		ld: Option<String>,
	},
}

#[derive(Debug, Subcommand)]
pub enum FindWhat {
	Path {
		#[arg(long)]
		ln: String,
		#[arg(long)]
		path: String,
	},
}

#[derive(Debug, Subcommand)]
pub enum GetWhat {
	Object {
		#[arg(long)]
		object: Option<String>,
		#[arg(long)]
		fc: Option<String>,
		#[arg(long)]
		csv: Option<String>,
	},
	Ln {
		#[arg(long)]
		ln: String,
		#[arg(long)]
		fc: Option<String>,
	},
	Ds {
		#[arg(long)]
		dataset: String,
	},
	Journal {
		#[arg(long)]
		ld: String,
		name: String,
		#[arg(long)]
		from: Option<String>,
		#[arg(long)]
		to: Option<String>,
		#[arg(long)]
		entry: Option<String>,
	},
}

#[derive(Debug, Subcommand)]
pub enum SetWhat {
	Object {
		#[arg(long)]
		object: String,
		#[arg(long)]
		fc: String,
		#[arg(long)]
		value: String,
		#[arg(long)]
		r#type: String,
		#[arg(long)]
		verify: bool,
	},
}

#[derive(Debug, Subcommand)]
pub enum SubscribeWhat {
	Report {
		#[arg(long)]
		ld: String,
		#[arg(long)]
		ln: String,
		#[arg(long)]
		report: String,
		/// `RP` or `BR`. Default `RP`.
		#[arg(long, default_value = "RP")]
		r#type: String,
		#[arg(long)]
		entry_id: Option<String>,
	},
}

#[derive(Debug, Subcommand)]
pub enum ControlWhat {
	Inspect {
		#[arg(long)]
		object: String,
	},
	Operate {
		#[arg(long)]
		object: String,
		#[arg(long)]
		value: String,
		#[arg(long)]
		r#type: String,
		#[arg(long)]
		yes: bool,
	},
}

#[derive(Debug, Subcommand)]
pub enum GooseWhat {
	List {
		#[arg(long)]
		scl: String,
	},
	Publish {
		#[arg(long)]
		iface: String,
		#[arg(long)]
		scl: String,
		#[arg(long)]
		gocb: String,
		#[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
		test: bool,
		#[arg(long)]
		live: bool,
	},
}

#[derive(Debug, Subcommand)]
pub enum SvWhat {
	Publish {
		#[arg(long)]
		iface: String,
		#[arg(long)]
		scl: String,
		#[arg(long)]
		svcb: String,
		#[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
		simulation: bool,
		#[arg(long)]
		live: bool,
	},
}
