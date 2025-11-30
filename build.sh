cd ../solid-csr-spa-template/
./deploy_to_be.sh
cd ../rust-be-template/
cargo upgrade --incompatible
cargo update
cargo +nightly build -Z build-std=std,core,alloc,panic_unwind --release
