bs:
    cargo run -- tp examples/black_scholes -o black-scholes-ag-rs --emit-manifest
v:
    cargo run -- -v tp examples/black_scholes -o black-scholes-ag-rs --emit-manifest
vv:
    cargo run -- -vv tp examples/black_scholes -o black-scholes-ag-rs --emit-manifest
install:
    cargo install --path .
