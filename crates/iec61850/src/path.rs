//! Object references.
//!
//! The command line writes `LD/LN.DO.DA` and passes the functional constraint
//! separately. MMS item names are `LN$FC$DO$DA`.

/// A client object reference split into the parts MMS needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliObject {
	/// Logical device name, the part before `/`.
	pub logical_device: String,
	/// Logical node name, the first component after `/`.
	pub logical_node: String,
	/// Data path under the logical node, still dot-separated.
	pub data_path: String,
	/// Functional constraint, such as `ST`.
	pub fc: String,
}

/// Why a command-line reference was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
	/// The reference has no `/` between the logical device and the rest.
	MissingSlash,
	/// The reference is not `LD/LN.DO...`, or the functional constraint is empty.
	Shape,
}

/// Parse `LD/LN.DO.DA` and attach `fc`.
pub fn parse_cli_object(reference: &str, fc: &str) -> Result<CliObject, PathError> {
	let (logical_device, rest) = reference.split_once('/').ok_or(PathError::MissingSlash)?;
	if logical_device.is_empty() || rest.is_empty() || rest.contains('/') || fc.is_empty() {
		return Err(PathError::Shape);
	}
	let (logical_node, data_path) = rest.split_once('.').ok_or(PathError::Shape)?;
	if logical_node.is_empty() || data_path.is_empty() {
		return Err(PathError::Shape);
	}
	Ok(CliObject {
		logical_device: logical_device.to_owned(),
		logical_node: logical_node.to_owned(),
		data_path: data_path.to_owned(),
		fc: fc.to_owned(),
	})
}

/// MMS item identifier: `LN$FC$DO$DA`.
pub fn object_to_mms(object: &CliObject) -> String {
	let data = object.data_path.replace('.', "$");
	format!("{}${}${}", object.logical_node, object.fc, data)
}

/// Dataset reference carried in a GOOSE or report frame: `{ld}/{ln}${name}`.
pub fn dataset_reference(logical_device: &str, logical_node: &str, name: &str) -> String {
	format!("{logical_device}/{logical_node}${name}")
}
