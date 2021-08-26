SERIAL_NRFDK := 683584342
SERIAL_PROTO1 := 801035168

OUTDIR2 := target/thumbv7em-none-eabihf

.PHONY: build release debug clean update deny program

release: build-nrfdk-release build-proto1-release

debug: build-nrfdk-debug build-proto1-debug

build: build-proto1-debug build-proto1-release build-nrfdk-debug build-nrfdk-release

ifeq ($(EXTRA_FEATURES),)
FEAT := 
else
FEAT := --features $(EXTRA_FEATURES)
endif

define build-template
build-$(1)-$(2):
	@echo "[31;1mBuilding board [$(1)] flavor [$(2)]...[37;0m"
	rm -f $(OUTDIR2)/$(2)/runner-$(1)
	cargo build $$(if $$(findstring release,$(2)),--release) --features board-$(1) $(FEAT) --color always 2> cargo_stderr.log; cat cargo_stderr.log
	arm-none-eabi-nm $(OUTDIR2)/$(2)/runner | grep ' [Tt] ' | sort | c++filt > symbols-$(1)-$(2).txt
	arm-none-eabi-objcopy -O ihex $(OUTDIR2)/$(2)/runner $(OUTDIR2)/$(2)/runner-$(1).ihex
	mv $(OUTDIR2)/$(2)/runner $(OUTDIR2)/$(2)/runner-$(1)
	arm-none-eabi-readelf -l $(OUTDIR2)/$(2)/runner-$(1) | grep LOAD
endef

$(eval $(call build-template,proto1,debug))
$(eval $(call build-template,proto1,release))
$(eval $(call build-template,nrfdk,debug))
$(eval $(call build-template,nrfdk,release))

clean:
	rm -rf target

update:
	cargo update

deny:
	cargo deny -c always check 2> deny.log

program:
	-nrfjprog -f NRF52 -s $(SERIAL_PROTO1) --sectorerase --program $(OUTDIR2)/release/runner-proto1.ihex
	-nrfjprog -f NRF52 -s $(SERIAL_NRFDK) --sectorerase --program $(OUTDIR2)/release/runner-nrfdk.ihex
