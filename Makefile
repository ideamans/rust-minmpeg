.PHONY: build build-release test test-golang clean

# Build debug version
build:
	cargo build

# Build release version
build-release:
	cargo build --release

# Run Rust tests
test:
	cargo test

# Build release and run Go tests
test-golang: build-release
	cd golang && go test -v

# Run all tests
test-all: test test-golang

# Clean build artifacts
clean:
	cargo clean
	cd golang && go clean -testcache
