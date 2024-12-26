#!/bin/bash

parent_dir="android_on_qemu"
aosp_ver="aosp-15.0.0_r8"
aosp_dir="$parent_dir/$aosp_ver"

android_kernel_ver="android15-6.6"
android_kernel_dir="$parent_dir/$android_kernel_ver"

cur_script_dir=$(dirname "$(realpath "$0")")
islet_dir=$(dirname "$cur_script_dir")

initramfs_path="$islet_dir/$android_kernel_dir/out/virtual_device_aarch64/dist/initramfs.img"
kernel_path="$islet_dir/$android_kernel_dir/out/virtual_device_aarch64/dist/Image"

function install_required_packages() {
    if [ -z "$(which repo)" ]; then
        sudo apt-get install repo
    fi
}

function build_aosp() {
    cd "$islet_dir" || exit

    # Create aosp directory and download AOSP sources
    if [ -d "$aosp_dir" ]; then
        echo "$aosp_dir already exists."

        echo "Changing directory to $aosp_dir..."
        cd $aosp_dir || exit 1 # if cd failed, exit with error code
    else
        echo "Creating directory $aosp_dir..."
        mkdir -p $aosp_dir

        echo "Changing directory to $aosp_dir..."
        cd $aosp_dir || exit 2 # if cd failed, exit with error code

        echo "Downloading AOSP sources..."
        repo init --partial-clone -b android-15.0.0_r8 -u https://android.googlesource.com/platform/manifest

        if ! repo sync -c -j8; then
            echo "ERROR: Download AOSP failed"
            exit 3
        fi
    fi

    if [ -f "out/host/linux-x86/bin/launch_cvd" ]; then
        echo "launch_cvd is exists. Skip building AOSP"
        return
    fi

    echo "Setting up build environment..."
    source build/envsetup.sh

    echo "Choosing a target..."
    lunch aosp_cf_arm64_only_phone-trunk_staging-userdebug

    echo "Building AOSP..."
    if ! m; then
        echo "ERROR: AOSP Build failed"
        exit 4
    fi

    echo "Go back to $islet_dir..."
    cd "$islet_dir" || exit 5 # if cd failed, exit with error code
}

function build_android_kernel() {
    cd "$islet_dir" || exit

    if [ ! -d "$android_kernel_dir" ]; then
        echo "Creating directory $android_kernel_dir..."
        mkdir -p "$android_kernel_dir"
    fi

    echo "Changing directory to $android_kernel_dir..."
    cd $android_kernel_dir || exit 1

    if [ ! -d "common" ]; then
        echo "Downloading Android Kernel sources..."
        repo init --partial-clone -u https://android.googlesource.com/kernel/manifest -b common-$android_kernel_ver
        if ! repo sync; then
            echo "ERROR: Download Android Kernel failed"
            exit 2
        fi
        echo "Replace common with cca patched kernel sources..."
        mv common backup_common
        git clone https://github.com/islet-project/3rd-android-kernel.git -b common-android15-6.6/cca-host/rmm-v1.0-eac5 --depth 1 --single-branch common
    fi

    if [ -f "$initramfs_path" ] && [ -f "$kernel_path" ]; then
        echo "Build is alread done. Skip building Android Kernel"
        return
    fi

    # Build
    echo "Building Android Kernel..."
    if ! tools/bazel run //common-modules/virtual-device:virtual_device_aarch64_dist_internal; then
        echo "ERROR: Android Kernel Build failed"
        exit 4
    fi

    echo "Check built images..."
    realpath $initramfs_path
    realpath $kernel_path

    echo "Go back to $islet_dir..."
    cd $islet_dir || exit 5
}

function run_qemu() {
    cd "$islet_dir" || exit
    # Go to the aosp source directory which you were built before
    echo "Changing directory to $aosp_dir to run qemu..."
    cd $aosp_dir || exit 1

    # Setup environment & select the target again
    echo "Setting up build environment..."
    . build/envsetup.sh
    lunch aosp_cf_arm64_only_phone-trunk_staging-userdebug

    # Run cuttlefish with cca support linux -> after start kernel, there is no logs..
    echo "Running Cuttlefish based by QEMU..."
    launch_cvd -vm_manager qemu_cli -enable_host_bluetooth false -report_anonymous_usage_stats=n \
        -initramfs_path $initramfs_path \
        -kernel_path $kernel_path
}

install_required_packages

build_aosp

build_android_kernel

run_qemu
