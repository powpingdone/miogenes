default: fmt

update:
    cargo update

spin: 
    cargo run -p mio-frontend 

drun: 
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 cargo run -p mio-backend 

rrun:
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 RUST_LOG="trace" cargo run -p mio-backend --release

prun:
    CARGO_PROFILE_RELEASE_DEBUG=true \
    RUSTFLAGS='--cfg tokio_unstable -C target-cpu=x86-64-v2' \
    cargo build -p mio-backend --release
    LD_LIBRARY_PATH="./target/release/" \
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 \
    target/release/mio-backend

clean:
    cargo clean

fmt:
    genemichaels -p
    cargo fmt
