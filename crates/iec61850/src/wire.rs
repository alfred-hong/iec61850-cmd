//! Encodings shared by MMS values and the later command line.
//!
//! Byte layouts follow IEC 61850-8-1: a boolean content octet is `0x00` or
//! `0x01`, a FLOAT32 content is five octets, and UtcTime is eight octets
//! with a 24-bit fraction.

/// ACSI EntryID is a fixed 8-octet string.
pub const ENTRY_ID_LEN: usize = 8;

/// Encode a boolean the way GOOSE and MMS Data send it.
pub fn encode_bool(value: bool) -> u8 {
	if value { 0x01 } else { 0x00 }
}

/// Decode a boolean content octet. Any non-zero octet is true.
pub fn decode_bool(octet: u8) -> bool {
	octet != 0
}

/// MMS floating-point content: exponent width 8, then IEEE-754 single precision.
pub fn encode_float32(value: f32) -> [u8; 5] {
	let bytes = value.to_be_bytes();
	[0x08, bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Eight-octet UtcTime. `fraction` is the 24-bit fraction of a second, units of 2⁻²⁴ s.
pub fn encode_utc(seconds: u32, fraction: u32, quality: u8) -> [u8; 8] {
	let seconds = seconds.to_be_bytes();
	let fraction = (fraction & 0x00FF_FFFF).to_be_bytes();
	[seconds[0], seconds[1], seconds[2], seconds[3], fraction[1], fraction[2], fraction[3], quality]
}

/// Thirteen-bit quality. Bit 0 is the leftmost bit. All zeros is good.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
	/// The 13 bits, right-aligned. Bit 12 of this value is standard bit 0.
	bits: u16,
}

impl Quality {
	/// validity = good, and every detail bit clear.
	pub fn good() -> Self {
		Self { bits: 0 }
	}

	/// Number of bits on the wire.
	pub fn bit_len(self) -> usize {
		13
	}

	/// Two octets, the 13 bits left-aligned. The low 3 bits of the second octet are unused.
	pub fn octets(self) -> [u8; 2] {
		(self.bits << 3).to_be_bytes()
	}

	/// Standard bit 11, the test flag.
	pub fn test(self) -> bool {
		self.bits & (1 << (12 - 11)) != 0
	}
}
