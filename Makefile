export IP=192.168.1.162

build-doorbell:
	cargo build --package doorbell --release --target armv7-unknown-linux-musleabihf

coppy-doorbell: build-doorbell
	scp target/armv7-unknown-linux-musleabihf/release/doorbell emil@$(IP):/home/emil

run-client:
	cargo run --package client --release
