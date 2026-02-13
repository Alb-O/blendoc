use crate::blend::bytes::Cursor;
use crate::blend::{BHead, BlendHeader};

#[test]
fn parses_v1_little_endian_bhead() {
	let header = BlendHeader::parse(b"BLENDER17-01v0500").expect("header parses");
	let mut bytes = Vec::new();
	bytes.extend_from_slice(b"TEST");
	bytes.extend_from_slice(&3_u32.to_le_bytes());
	bytes.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
	bytes.extend_from_slice(&16_i64.to_le_bytes());
	bytes.extend_from_slice(&2_i64.to_le_bytes());

	let mut cursor = Cursor::new(&bytes);
	let head = BHead::parse(&mut cursor, header).expect("bhead parses");
	assert_eq!(head.code, *b"TEST");
	assert_eq!(head.sdna_nr, 3);
	assert_eq!(head.old, 0x1122_3344_5566_7788);
	assert_eq!(head.len, 16);
	assert_eq!(head.nr, 2);
}

#[test]
fn parses_legacy_big_endian_bhead() {
	let header = BlendHeader::parse(b"BLENDER_V248").expect("header parses");
	let mut bytes = Vec::new();
	bytes.extend_from_slice(b"TEST");
	bytes.extend_from_slice(&12_i32.to_be_bytes());
	bytes.extend_from_slice(&0x99AA_BBCC_u32.to_be_bytes());
	bytes.extend_from_slice(&7_u32.to_be_bytes());
	bytes.extend_from_slice(&1_i32.to_be_bytes());

	let mut cursor = Cursor::new(&bytes);
	let head = BHead::parse(&mut cursor, header).expect("legacy bhead parses");
	assert_eq!(head.code, *b"TEST");
	assert_eq!(head.sdna_nr, 7);
	assert_eq!(head.old, 0x99AA_BBCC);
	assert_eq!(head.len, 12);
	assert_eq!(head.nr, 1);
}
