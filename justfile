default: build

gen:
    flutter_rust_bridge_codegen --rust-input frontend/glue/src/api.rs --dart-output frontend/lib/bridge_generated.dart

build: gen
    cd frontend && flutter build 
    cargo build -p mio-backend

drun: build
    cargo run -p mio-backend 