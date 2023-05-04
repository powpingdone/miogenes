default: build

build: 
    cd frontend && trunk build
    cargo build -p mio-backend

release:
    cd frontend && trunk build --release
    cargo build -p mio-backend --release