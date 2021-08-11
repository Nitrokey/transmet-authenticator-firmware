EXE := target/thumbv7em-none-eabihf/debug/runner
OPTIONS := --features board-proto1

.PHONY: build clean update deny

build:
	cargo build $(OPTIONS)
	arm-none-eabi-nm $(EXE) | grep ' [Tt] ' | sort | c++filt > symbols.txt
	arm-none-eabi-objcopy -O ihex $(EXE) $(EXE).ihex

clean:
	rm -rf target

update:
	cargo update

deny:
	cargo deny -c always check 2> deny.log

