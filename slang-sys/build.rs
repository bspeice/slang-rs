extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn do_link_env() -> Option<PathBuf> {
	println!("cargo:rerun-if-env-changed=SLANG_DIR");
	Some(env::var("SLANG_DIR").ok()?.into())
}

fn do_link_cmake() -> Option<PathBuf> {
	let cmake_output = cmake::Config::new("./slang")
		// https://github.com/shader-slang/slang/issues/5832#issuecomment-2533324982
		.define("CMAKE_SKIP_INSTALL_RULES", "ON")
		.build_target("slang")
		.build();

	for build_type in ["Debug", "Release"] {
		let slang_dir = cmake_output.join("build").join(build_type);
		if slang_dir.exists() {
			return Some(slang_dir);
		}
	}

	None
}

fn main() {
	println!("cargo:rerun-if-changed=build.rs");

	let slang_dir = do_link_env()
		.or_else(do_link_cmake)
		.expect("Unable to locate slang");
	println!("cargo:rustc-link-lib=dylib=slang");
	println!("cargo:rustc-link-search=native={}", slang_dir.join("lib").display());

	bindgen::builder()
		.header(slang_dir.join("include/slang.h").to_str().unwrap())
		.clang_arg("-v")
		.clang_arg("-xc++")
		.clang_arg("-std=c++17")
		.allowlist_function("spReflection.*")
		.allowlist_function("slang_.*")
		.allowlist_type("slang.*")
		.allowlist_var("SLANG_.*")
		.with_codegen_config(
			bindgen::CodegenConfig::FUNCTIONS
				| bindgen::CodegenConfig::TYPES
				| bindgen::CodegenConfig::VARS,
		)
		.parse_callbacks(Box::new(ParseCallback {}))
		.default_enum_style(bindgen::EnumVariation::Rust {
			non_exhaustive: false,
		})
		.constified_enum("SlangProfileID")
		.constified_enum("SlangCapabilityID")
		.vtable_generation(true)
		.layout_tests(false)
		.derive_copy(true)
		.generate()
		.expect("Couldn't generate bindings.")
		.write_to_file(format!("{}/bindings.rs", env::var("OUT_DIR").unwrap()))
		.expect("Couldn't write bindings.");
}

#[derive(Debug)]
struct ParseCallback {}

impl bindgen::callbacks::ParseCallbacks for ParseCallback {
	fn enum_variant_name(
		&self,
		enum_name: Option<&str>,
		original_variant_name: &str,
		_variant_value: bindgen::callbacks::EnumVariantValue,
	) -> Option<String> {
		let enum_name = enum_name?;

		// Map enum names to the part of their variant names that needs to be trimmed.
		// When an enum name is not in this map the code below will try to trim the enum name itself.
		let mut map = std::collections::HashMap::new();
		map.insert("SlangMatrixLayoutMode", "SlangMatrixLayout");
		map.insert("SlangCompileTarget", "Slang");

		let trim = map.get(enum_name).unwrap_or(&enum_name);
		let new_variant_name = pascal_case_from_snake_case(original_variant_name);
		let new_variant_name = new_variant_name.trim_start_matches(trim);
		Some(new_variant_name.to_string())
	}
}

/// Converts `snake_case` or `SNAKE_CASE` to `PascalCase`.
/// If the input is already in `PascalCase` it will be returned as is.
fn pascal_case_from_snake_case(snake_case: &str) -> String {
	let mut result = String::new();

	let should_lower = snake_case
		.chars()
		.filter(|c| c.is_alphabetic())
		.all(|c| c.is_uppercase());

	for part in snake_case.split('_') {
		for (i, c) in part.chars().enumerate() {
			if i == 0 {
				result.push(c.to_ascii_uppercase());
			} else if should_lower {
				result.push(c.to_ascii_lowercase());
			} else {
				result.push(c);
			}
		}
	}

	result
}
