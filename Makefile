all: ./target/release/favicon-generator

./target/release/favicon-generator: $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | sed 's/ /\\ /g')
	PWD=$$(pwd)
	cd $$MAGICK_PATH && bash build.sh
	cd $$PWD
	IMAGE_MAGICK_LIB_DIRS="$$MAGICK_PATH/linux/lib" IMAGE_MAGICK_INCLUDE_DIRS="$$MAGICK_PATH/linux/include/ImageMagick-7" IMAGE_MAGICK_STATIC=1 IMAGE_MAGICK_LIBS=z:ltdl:bz2:uuid:jbig:expat:fontconfig:freetype:gettextpo:harfbuzz:iconv:jpeg:lzma:openjp2:png16:python2.7:tiff:webpmux:webpdemux:webp:xml2:MagickWand-7.Q16HDRI:MagickCore-7.Q16HDRI cargo build --release
	strip ./target/release/favicon-generator
	
install:
	$(MAKE)
	sudo cp ./target/release/favicon-generator /usr/local/bin/favicon-generator
	sudo chown root: /usr/local/bin/favicon-generator
	sudo chmod 0755 /usr/local/bin/favicon-generator

uninstall:
	sudo rm /usr/local/bin/favicon-generator

test:
	cargo test --verbose
	
clean:
	cargo clean
