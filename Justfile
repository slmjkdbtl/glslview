# wengwengweng

set shell := ["fish", "-c"]
name := "glslview"
version := "0.0.0"

@run +args="":
	cargo run --release -- {{args}}

@build:
	cargo build --release

@macos: build
	icns icon.png icon.icns
	rm -rf dist/{{name}}.app
	rm -rf dist/{{name}}_v{{version}}_mac.tar.gz
	upx target/release/{{name}} -o {{name}}
	packapp {{name}} --name {{name}} --icon icon.icns -o dist/{{name}}.app
	tar czf dist/{{name}}_v{{version}}_mac.tar.gz dist/{{name}}.app
	rm {{name}}
	rm icon.icns

@doc crate:
	cargo doc --no-deps --open -p {{crate}}

@update:
	cargo update

@bloat:
	cargo bloat --release --crates

@loc:
	loc

@checkdep:
	cargo outdated --root-deps-only

