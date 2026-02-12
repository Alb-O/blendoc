use crate::blend::{BlendError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStep {
	Field(String),
	Index(usize),
}

#[derive(Debug, Clone)]
pub struct FieldPath {
	pub steps: Vec<PathStep>,
}

impl FieldPath {
	pub fn parse(input: &str) -> Result<Self> {
		if input.is_empty() {
			return Err(BlendError::InvalidFieldPath { path: input.to_owned() });
		}

		let bytes = input.as_bytes();
		let mut idx = 0_usize;
		let mut steps = Vec::new();

		while idx < bytes.len() {
			let start = idx;
			while idx < bytes.len() {
				let byte = bytes[idx];
				if byte.is_ascii_alphanumeric() || byte == b'_' {
					idx += 1;
				} else {
					break;
				}
			}

			if idx == start {
				return Err(BlendError::InvalidFieldPath { path: input.to_owned() });
			}

			steps.push(PathStep::Field(input[start..idx].to_owned()));

			while idx < bytes.len() && bytes[idx] == b'[' {
				idx += 1;
				let n_start = idx;
				while idx < bytes.len() && bytes[idx].is_ascii_digit() {
					idx += 1;
				}
				if idx == n_start || idx >= bytes.len() || bytes[idx] != b']' {
					return Err(BlendError::InvalidFieldPath { path: input.to_owned() });
				}

				let number = input[n_start..idx]
					.parse::<usize>()
					.map_err(|_| BlendError::InvalidFieldPath { path: input.to_owned() })?;
				steps.push(PathStep::Index(number));
				idx += 1;
			}

			if idx < bytes.len() {
				if bytes[idx] != b'.' {
					return Err(BlendError::InvalidFieldPath { path: input.to_owned() });
				}
				idx += 1;
				if idx >= bytes.len() {
					return Err(BlendError::InvalidFieldPath { path: input.to_owned() });
				}
			}
		}

		Ok(Self { steps })
	}
}
