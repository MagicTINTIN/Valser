#!/usr/bin/env bash
set -euo pipefail

target=${1:-}
args=()

case $target in
    linux-x64) args=(--target linux --platform linux/amd64) ;;
    linux-arm64) args=(--target linux --platform linux/arm64) ;;
    windows-x64) args=(--target windows --build-arg ARCH=x86_64) ;;
    windows-arm64) args=(--target windows --build-arg ARCH=aarch64) ;;
    android)
        args=(--target android)
        if [[ -n ${ANDROID_KEYSTORE:-} ]]; then
            args+=(--secret "id=keystore,src=$ANDROID_KEYSTORE")
            args+=(--secret id=keystore_password,env=ANDROID_KEYSTORE_PASSWORD)
        fi
        ;;
    *)
        echo "usage: $0 linux-x64|linux-arm64|windows-x64|windows-arm64|android" >&2
        exit 1
        ;;
esac

docker buildx build -f docker/Dockerfile "${args[@]}" --output "dist/$target" .
