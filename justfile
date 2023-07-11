default: gen

gen:
    flutter_rust_bridge_codegen --rust-input frontend/glue/src/api.rs --dart-output frontend/lib/bridge_generated.dart

spin: gen 
    cd frontend && flutter run

drun: 
    cargo run -p mio-backend 

rrun:
    RUST_LOG="trace" cargo run -p mio-backend --release

fmt:
    genemichaels -p
    cargo fmt
    dart format frontend/