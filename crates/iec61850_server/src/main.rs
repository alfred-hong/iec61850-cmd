fn main() {
	let code = iec61850_cmd_server::run_to_stdio();
	std::process::exit(code);
}
