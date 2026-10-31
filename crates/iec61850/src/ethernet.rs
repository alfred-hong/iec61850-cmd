//! Ethertype PDU header shared by GOOSE (`0x88B8`) and sampled values (`0x88BA`).
//!
//! After the Ethertype the header is APPID, Length, Reserved1, Reserved2,
//! each two octets, then the APDU. Length counts from APPID through the end
//! of the APDU, so it is `8 + m`.

/// `0x0000` means the APPID was not configured.
pub const APPID_UNCONFIGURED: u16 = 0x0000;

/// Simulation flag: the high bit of Reserved1.
pub const SIMULATION_BIT: u16 = 0x8000;

/// Encode the eight octets that follow the Ethertype.
///
/// `apdu_len` is `m`, the APDU size in octets. The simulation bit is set only
/// when `simulation` is true; the other Reserved1 bits and Reserved2 stay 0.
pub fn apdu_header(appid: u16, apdu_len: usize, simulation: bool) -> [u8; 8] {
	let length = (8 + apdu_len) as u16;
	let reserved1 = if simulation { SIMULATION_BIT } else { 0 };
	let mut header = [0_u8; 8];
	header[0..2].copy_from_slice(&appid.to_be_bytes());
	header[2..4].copy_from_slice(&length.to_be_bytes());
	header[4..6].copy_from_slice(&reserved1.to_be_bytes());
	header
}
