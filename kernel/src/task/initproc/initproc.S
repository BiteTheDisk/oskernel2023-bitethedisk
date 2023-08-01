.align 3
.section .data
.globl initproc_entry
.globl initproc_tail
initproc_entry:
	.incbin "../crates/libd/target/riscv64gc-unknown-none-elf/release/initproc"
initproc_tail:

.align 3
.section .data
.globl busybox_entry
.globl busybox_tail
busybox_entry:
	.incbin "src/task/initproc/busybox"
busybox_tail:

.align 3
.section .data
.globl test_all2_entry
.globl test_all2_tail
test_all2_entry:
	.incbin "src/task/initproc/test_all2.sh"
test_all2_tail:
