//! Command line shape from PLAN_server.md.

use clap::{Parser, Subcommand};

/// IEC 61850 server.
#[derive(Debug, Parser)]
#[command(name = "iec61850_server", about = "IEC 61850 server")]
pub struct Cli {
	/// `text`, `json`, or `jsonl`. Not used by `start` yet.
	#[arg(long, global = true)]
	pub format: Option<String>,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Load an SCL file and listen for MMS clients.
	Start {
		/// SCL path. Required. ICD, CID, and SCD are expected.
		#[arg(long)]
		scl: String,

		/// IED to serve. Default is the first IED that has a logical device.
		#[arg(long)]
		ied_name: Option<String>,

		/// JSON file that only overrides existing leaf values.
		#[arg(long)]
		values: Option<String>,

		/// Listen address. Default `0.0.0.0`.
		#[arg(long, default_value = "0.0.0.0")]
		host: String,

		/// Listen port. Default `102`. `0` lets the system pick a port.
		#[arg(long, default_value_t = 102)]
		port: u16,

		/// Max simultaneous associations. Default 5.
		#[arg(long, default_value_t = 5)]
		max_connections: u32,

		/// After listen succeeds, print one ready JSON line on stdout.
		#[arg(long)]
		ready_json: bool,
	},
	/// Print the server version.
	Version,
}
