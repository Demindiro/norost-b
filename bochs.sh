#!/bin/sh

./mkiso.sh || exit $?

bochs -q
