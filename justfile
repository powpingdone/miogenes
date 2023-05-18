default: build

build: 
    cd frontend && trunk build
    cargo build -p mio-backend

release:
    cd frontend && trunk build --release
    cargo build -p mio-backend --release

drun: build
    cargo run -p mio-backend

run: release
    cargo run -p mio-backend --release