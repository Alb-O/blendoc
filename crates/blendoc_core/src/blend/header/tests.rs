use crate::blend::{BlendError, BlendHeader, Endianness};

#[test]
fn parses_large_bhead8_header() {
	let header = BlendHeader::parse(b"BLENDER17-01v0500").expect("header parses");
	assert_eq!(header.header_size, 17);
	assert_eq!(header.format_version, 1);
	assert_eq!(header.version, 500);
	assert_eq!(header.pointer_size, 8);
	assert_eq!(header.endianness, Endianness::Little);
	assert_eq!(header.bhead_layout_label(), "large_bhead8");
}

#[test]
fn rejects_non_large_bhead8_header_size_marker() {
	let err = BlendHeader::parse(b"BLENDER18-01v0500X").expect_err("non-17 size marker should fail");
	assert!(matches!(err, BlendError::UnsupportedPointerSize { header_size: 18 }));
}

#[test]
fn parses_legacy_little_endian_header() {
	let header = BlendHeader::parse(b"BLENDER-v302").expect("legacy header parses");
	assert_eq!(header.header_size, BlendHeader::LEGACY_SIZE);
	assert_eq!(header.format_version, BlendHeader::LEGACY_FORMAT_VERSION);
	assert_eq!(header.version, 302);
	assert_eq!(header.pointer_size, 8);
	assert_eq!(header.endianness, Endianness::Little);
	assert_eq!(header.bhead_layout_label(), "legacy");
}

#[test]
fn parses_legacy_big_endian_header() {
	let header = BlendHeader::parse(b"BLENDER_V248").expect("legacy big-endian header parses");
	assert_eq!(header.header_size, BlendHeader::LEGACY_SIZE);
	assert_eq!(header.format_version, BlendHeader::LEGACY_FORMAT_VERSION);
	assert_eq!(header.version, 248);
	assert_eq!(header.pointer_size, 4);
	assert_eq!(header.endianness, Endianness::Big);
}
