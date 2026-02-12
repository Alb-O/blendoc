/// Parsed SDNA field declarator details.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldDecl<'a> {
	/// Identifier portion of the declarator.
	pub ident: &'a str,
	/// Pointer nesting depth (`*`, `**`, ...).
	pub ptr_depth: u8,
	/// Flattened inline array element count.
	pub inline_array: usize,
	/// Whether declarator looks like a function pointer.
	pub is_func_ptr: bool,
	/// Whether pointer stars were parenthesized.
	pub is_paren_ptr: bool,
}

/// Parse SDNA declarator text into normalized pointer/array metadata.
pub(crate) fn parse_field_decl(raw: &str) -> FieldDecl<'_> {
	let trimmed = raw.trim();
	let mut decl = FieldDecl {
		ident: trimmed,
		ptr_depth: 0,
		inline_array: 1,
		is_func_ptr: trimmed.contains(")("),
		is_paren_ptr: false,
	};

	if let Some(start) = trimmed.find("(*") {
		let after = &trimmed[start + 2..];
		if let Some(close_idx) = after.find(')') {
			let inside = &after[..close_idx];
			let stars = inside.chars().take_while(|c| *c == '*').count();
			decl.ptr_depth = (stars as u8).saturating_add(1);
			let ident = inside.trim_start_matches('*').trim();
			if !ident.is_empty() {
				decl.ident = ident;
			}
			decl.is_paren_ptr = true;
			decl.inline_array = 1;
			return decl;
		}
	}

	let stars = trimmed.chars().take_while(|c| *c == '*').count();
	decl.ptr_depth = stars as u8;
	let mut tail = &trimmed[stars..];

	let ident_end = tail.find('[').unwrap_or(tail.len());
	let ident = tail[..ident_end].trim();
	if !ident.is_empty() {
		decl.ident = ident;
	}

	tail = &tail[ident_end..];
	if !decl.is_paren_ptr && !decl.is_func_ptr {
		let mut total = 1_usize;
		while let Some(start) = tail.find('[') {
			let Some(end) = tail[start + 1..].find(']') else {
				break;
			};
			let end = start + 1 + end;
			let dim = tail[start + 1..end].trim().parse::<usize>().unwrap_or(1);
			total = total.saturating_mul(dim);
			tail = &tail[end + 1..];
		}
		decl.inline_array = total;
	}

	decl
}

#[cfg(test)]
mod tests {
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
}
