#!/usr/bin/env bash
set -euo pipefail

unsigned=src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
keystore=/run/secrets/keystore

if [[ ! -f $keystore ]]; then
    echo "No keystore provided, signing with a throwaway debug key"
    keystore=/tmp/debug.jks
    export KEYSTORE_PASSWORD=android
    keytool -genkeypair -keystore $keystore -storepass android -alias debug \
        -keyalg RSA -validity 10000 -dname CN=debug
fi

"$ANDROID_HOME"/build-tools/36.0.0/apksigner sign --v4-signing-enabled false --ks $keystore --ks-pass env:KEYSTORE_PASSWORD \
    --out /out/valser.apk $unsigned
