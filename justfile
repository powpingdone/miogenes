default: gen fmt

update:
    cargo update
    flutter pub upgrade

gen:
    flutter_rust_bridge_codegen --rust-input frontend/glue/src/api.rs --dart-output frontend/lib/bridge_generated.dart

spin: gen 
    cd frontend && flutter run

drun: 
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 cargo run -p mio-backend 

rrun:
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 RUST_LOG="trace" cargo run -p mio-backend --release

prun:
    CARGO_PROFILE_RELEASE_DEBUG=true \
    RUSTFLAGS='--cfg tokio_unstable -C force-frame-pointers=y' \
    LD_LIBRARY_PATH="./target/release/" \
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 \
    cargo flamegraph -p mio-backend

clean:
    cargo clean
    cd frontend && flutter clean && flutter pub get
    rm -r frontend/android/app/src/main/jniLibs/*/libmio_glue.so 

fmt:
    genemichaels -p
    cargo fmt
    dart format frontend/