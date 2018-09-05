all: ./target/release/favicon-generator

./target/release/favicon-generator: $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | sed 's/ /\\ /g')
	cargo build --release
	strip ./target/release/favicon-generator
	
install:
	$(MAKE)
	sudo cp ./target/release/favicon-generator /usr/local/bin/favicon-generator
	sudo chown root. /usr/local/bin/favicon-generator
	sudo chmod 0755 /usr/local/bin/favicon-generator
	
test:
	cargo test --verbose
	
clean:
	cargo clean
