default: build

gen:
    flutter_rust_bridge_codegen --rust-input frontend/glue/src/api.rs --dart-output frontend/lib/bridge_generated.dart

build: gen
    mkdir -p frontend/build/lib
    cargo build -p mio-backend

spin: build
    cd frontend && flutter run

drun: build
    cargo run -p mio-backend 