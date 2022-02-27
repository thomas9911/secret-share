cd frontend
trunk clean
trunk build --release
cd ..
cargo build --release --bin secret-share
