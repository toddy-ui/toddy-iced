# plushie-iced - Development Tasks
#
# Run `just` to see available recipes.
# Run `just preflight` before pushing to catch CI failures locally.

export RUSTFLAGS := "-D warnings"
export ICED_TEST_BACKEND := "tiny-skia"

default:
    @just --list

# === CI Preflight ===

preflight: check check-wasm check-widget lint fmt doc audit test
    @echo ""
    @echo "All preflight checks passed!"

# === Individual Checks ===

check:
    cargo check --workspace --all-targets

# Mirrors `.github/workflows/check.yml` wasm job. Requires the
# `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`.
check-wasm:
    RUSTFLAGS="--cfg=web_sys_unstable_apis" cargo check --package plushie-iced --target wasm32-unknown-unknown

# Mirrors `.github/workflows/check.yml` widget job: standalone
# `plushie-iced-widget` crate compiles with feature subset.
check-widget:
    cargo check --package plushie-iced-widget --features image,svg,canvas

lint:
    cargo lint

fmt:
    cargo fmt --all -- --check --verbose

# Checks docs build. CI uses nightly with `--cfg docsrs` (see `doc-rs`);
# this recipe is the cheaper local sanity check.
doc:
    cargo doc --no-deps --workspace

# Mirrors `.github/workflows/document.yml`. Requires a nightly toolchain.
doc-rs:
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features \
        -p plushie-iced-beacon \
        -p plushie-iced-core \
        -p plushie-iced-debug \
        -p plushie-iced-devtools \
        -p plushie-iced-futures \
        -p plushie-iced-graphics \
        -p plushie-iced-highlighter \
        -p plushie-iced-renderer \
        -p plushie-iced-runtime \
        -p plushie-iced-tiny-skia \
        -p plushie-iced-wgpu \
        -p plushie-iced-widget \
        -p plushie-iced-winit \
        -p plushie-iced-test \
        -p plushie-iced

test:
    cargo test --verbose --workspace
    cargo test --verbose --workspace -- --ignored
    cargo test --verbose --workspace --all-features

# === Build Variants ===

build:
    cargo build --workspace

build-release:
    cargo build --release --workspace

# === Development Helpers ===

format:
    cargo fmt --all

lint-fix:
    cargo lint-fix

test-filter pattern:
    cargo test --workspace -- {{pattern}}

test-crate crate:
    cargo test -p {{crate}}

clean:
    cargo clean

docs:
    cargo doc --workspace --open

# === Watch Mode ===

watch-check:
    cargo watch -x 'check --workspace --all-targets'

watch-test:
    cargo watch -x 'test --workspace'

# === Dependency Health ===

audit:
    cargo audit

outdated:
    cargo outdated --workspace
