//! Wire shapes the command line depends on. Expected bytes come from IEC 61850-8-1, not from the encoder.

use iec61850_cmd::ethernet::{APPID_UNCONFIGURED, apdu_header};
use iec61850_cmd::path::{dataset_reference, object_to_mms, parse_cli_object};
use iec61850_cmd::wire::{
	ENTRY_ID_LEN, Quality, decode_bool, encode_bool, encode_float32, encode_utc,
};

#[test]
fn cli_object_becomes_mms_item() {
	let object = parse_cli_object("HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal", "ST").unwrap();
	assert_eq!(object.logical_device, "HV_3_AH_04CTRL");
	assert_eq!(object_to_mms(&object), "BIGGIO1$ST$SPCSO10$stVal");
}

#[test]
fn dataset_reference_keeps_the_dollar() {
	assert_eq!(dataset_reference("LD0", "LLN0", "dsGoose"), "LD0/LLN0$dsGoose");
}

#[test]
fn boolean_octet_is_zero_or_one_and_any_nonzero_is_true() {
	assert_eq!(encode_bool(false), 0x00);
	assert_eq!(encode_bool(true), 0x01);
	assert!(!decode_bool(0x00));
	assert!(decode_bool(0x01));
	assert!(decode_bool(0x02));
}

#[test]
fn float32_mms_content_is_five_octets() {
	// 1.0 IEEE-754 is 3F 80 00 00. MMS prepends exponent width 8 (tissue 817).
	assert_eq!(encode_float32(1.0), [0x08, 0x3F, 0x80, 0x00, 0x00]);
}

#[test]
fn quality_good_is_thirteen_zero_bits() {
	let quality = Quality::good();
	assert_eq!(quality.bit_len(), 13);
	assert_eq!(quality.octets(), [0x00, 0x00]);
	assert!(!quality.test());
}

#[test]
fn utc_time_keeps_twenty_four_bit_fraction_and_quality_in_the_last_octet() {
	// 1 second + 0 fraction. Publisher quality: ClockNotSynchronized and accuracy unspecified (31).
	let encoded = encode_utc(1, 0, 0x3F);
	assert_eq!(encoded, [0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3F]);
}

#[test]
fn entry_id_is_eight_octets() {
	assert_eq!(ENTRY_ID_LEN, 8);
}

#[test]
fn goose_header_length_counts_from_appid_and_simulation_is_the_high_bit() {
	let header = apdu_header(0x0001, 20, true);
	assert_eq!(&header[0..2], &[0x00, 0x01]);
	assert_eq!(&header[2..4], &[0x00, 28]);
	assert_eq!(&header[4..6], &[0x80, 0x00]);
	assert_eq!(&header[6..8], &[0x00, 0x00]);
	assert!(APPID_UNCONFIGURED == 0x0000);
}
