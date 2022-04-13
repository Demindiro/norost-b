#!/bin/sh

. ./env.sh

./mkkernel.sh || exit $?
./mkboot.sh || exit $?

set -e

TARGET_BOOT=i686-unknown-none-norostbkernel
TARGET_KERNEL=x86_64-unknown-none-norostbkernel
TARGET_USER=x86_64-unknown-norostb

mkdir -p isodir/boot/grub isodir/drivers
cp target/$TARGET_KERNEL/release/nora isodir/boot/nora
cp target/$TARGET_BOOT/release/noraboot isodir/boot/noraboot
cp boot/$ARCH/grub/grub.cfg isodir/boot/grub/grub.cfg

(cd drivers/fs_fat && cargo build --release --target $TARGET_USER)
cp target/$TARGET_USER/release/driver_fs_fat isodir/drivers/fs_fat
(cd drivers/virtio_block && cargo build --release --target $TARGET_USER)
cp target/$TARGET_USER/release/driver_virtio_block isodir/drivers/virtio_block
(cd drivers/virtio_net && cargo build --release --target $TARGET_USER)
cp target/$TARGET_USER/release/driver_virtio_net isodir/drivers/virtio_net
(cd base/minish && cargo build --release --target $TARGET_USER)
cp target/$TARGET_USER/release/minish isodir/drivers/minish

# Note: make sure grub-pc-bin is installed! Otherwise QEMU may hang on
# "Booting from disk" or return error code 0009
grub-mkrescue -o norost.iso isodir \
	--locales= \
	--fonts= \
	--install-modules="multiboot2 normal" \
	--modules= \
	--compress=xz
