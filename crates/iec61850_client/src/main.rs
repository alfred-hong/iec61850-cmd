fn main() {
	let code = iec61850_cmd_client::run_to_stdio();
	std::process::exit(code);
}
