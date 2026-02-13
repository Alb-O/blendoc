use super::parse_field_decl;

#[test]
fn paren_pointer_depth_is_detected() {
	let decl = parse_field_decl("(*next)");
	assert_eq!(decl.ptr_depth, 1);
	assert_eq!(decl.ident, "next");
	assert_eq!(decl.inline_array, 1);
}

#[test]
fn double_paren_pointer_depth_is_detected() {
	let decl = parse_field_decl("(**func)");
	assert_eq!(decl.ptr_depth, 2);
	assert_eq!(decl.ident, "func");
}

#[test]
fn zero_sized_array_is_preserved() {
	let decl = parse_field_decl("weights[0]");
	assert_eq!(decl.inline_array, 0);
}
