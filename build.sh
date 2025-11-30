git reset --hard && git pull
cd ../solid-csr-spa-template/
git reset --hard && git pull
./deploy_to_be.sh
cd ../rust-be-template/
cargo upgrade --incompatible
cargo update
cargo +nightly build -Z build-std=std,core,alloc,panic_unwind --release
