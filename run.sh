#!/bin/sh

./mkiso.sh || exit $?
make disk0 || exit $?

cpu="--enable-kvm -cpu host"

[ -z ${CPU+x} ] || cpu="-cpu $CPU"

exec qemu-system-x86_64 \
	$cpu \
	-machine q35 \
	-drive format=raw,media=cdrom,file=norost.iso \
	-serial mon:stdio \
	-drive file=disk0,format=raw,if=none,id=disk0 \
	-device virtio-blk-pci,drive=disk0 \
	-netdev user,id=net0,hostfwd=tcp::5555-:333 \
	-device virtio-net-pci,netdev=net0 \
	"$@"
