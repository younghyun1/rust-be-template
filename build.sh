cd ../solid-csr-spa-template/
./deploy_to_be.sh
cargo upgrade --incompatible
cargo update
cargo +nightly build -Z build-std=std,core,alloc,panic_unwind --release
