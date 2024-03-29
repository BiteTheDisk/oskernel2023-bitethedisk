##################################################################
#                         Configuration                          #
##################################################################
TARGET := riscv64gc-unknown-none-elf

KERNEL_ELF := target/$(TARGET)/release/kernel
KERNEL_ELF_DBG := target/$(TARGET)/debug/kernel

QEMU = qemu-system-riscv64
GDB = riscv64-unknown-elf-gdb

SDCARD = ../workspace/sdcard.img
SDCARD_BAK = ../workspace/sdcard.img.bak
NCPU = 2

RUST_LOG := debug

# Available features: no-page-cache,static-busybox,inode-drop,fu740,dev,hash-inode-cache,multi-harts
FEATURES ?= --features static-busybox

DEV ?= false

BOARD ?= qemu

define run-board-qemu
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF) \
		-m 128M \
		-nographic \
		-smp 2 \
		-bios default \
		-drive file=$(SDCARD),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-device virtio-net-device,netdev=net \
		-netdev user,id=net
endef
define run-board-fu740
	 @(QEMU) \
		-machine virt \
		-kernel $(KERNEL_ELF) \
		-m 2G \
		-nographic \
		-smp 4 \
		-drive file=sdcard.img,if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
endef
##################################################################
#                        Run straightly                          #
##################################################################
initproc:
	@cd ../crates/libd && cargo build --release
sdcard:
	@rm -f $(SDCARD)
	@cp $(SDCARD_BAK) $(SDCARD)

kernel: initproc
	@cargo build --release --features $(BOARD) --offline

kernel-dev: initproc
ifeq ($(DEV), true)
	@RUST_LOG=$(RUST_LOG) cargo build --release --offline $(FEATURES)
else
	@RUST_LOG=error cargo build --release --offline $(FEATURES)
endif

run: kernel-dev sdcard
ifeq ($(BOARD), qemu)
	@$(run-board-qemu)
else ifeq ($(BOARD), fu740)
	@$(run-board-fu740)
else
	@$(error "Platform not supported (required $(BOARD))")
endif

##################################################################
#                      Debug build & run.                        #
##################################################################
kernel-debug: initproc
	@RUST_LOG=$(RUST_LOG) cargo build

debug-server: kernel-debug sdcard
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF_DBG) \
		-m 128M \
		-nographic \
		-smp $(NCPU) \
		-drive file=$(SDCARD),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S

debug:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF_DBG)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'
