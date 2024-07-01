default: fmt

update:
    cargo update

spin: 
    

drun: 
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 cargo run -p mio-backend 

rrun:
    DATA_DIR="./files" IP_ADDR="127.0.0.1" PORT=8081 SIGNUP_ENABLED=1 RUST_LOG="trace" cargo run -p mio-backend --release

clean:
    cargo clean
    rm files/*

fmt:
    find | grep -v './target' | grep '\.rs' | xargs genemichaels
    cargo fmt
