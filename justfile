diehard file:
    dieharder -a -g 201 -f {{file}}

build arg:
	cargo build {{arg}}
	cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 build {{arg}}
