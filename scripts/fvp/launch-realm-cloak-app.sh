#!/bin/sh

cd /shared

#./configure-net.sh &

./lkvm run \
	--debug \
	--realm \
	--measurement-algo="sha256" \
	--disable-sve \
	--console serial \
	--irqchip=gicv3 \
	--network virtio \
	--realm-pv="no_shared_region" \
	--vcpu-affinity 0-1 \
	-m 256M \
	-c 1 \
	-k linux.realm \
	-i rootfs-realm.cpio.gz \
	-p "earlycon=ttyS0 printk.devkmsg=on no_shared_region=on"
