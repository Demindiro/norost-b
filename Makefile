# There's a bug in cargo that causes panics when using `forced-target`
# Use Makefiles as workaround for now
build: kernel boot

kernel:
	cargo b --bin nora

boot:
	cargo b --bin noraboot

run:
	./run.sh

disk0:
	fallocate -l $$((32 * 512)) $@

.PHONY: kernel boot run
