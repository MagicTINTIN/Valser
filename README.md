# Valser
Rust music player with niche and or useless features.

- `core`: player logic (playlist, audio, library storage)
- `src-tauri`: Tauri app, commands and the playback thread
- `web`: plain HTML and JS interface

## Dev

Linux needs `webkit2gtk-4.1`, `dbus` and `alsa` development packages.

```sh
cargo run
```

### Tests
```sh
cargo test --workspace
```

## Builds

Requires Docker. Outputs go to `dist/<target>/`.

```sh
./build.sh linux-x64
./build.sh linux-arm64
./build.sh windows-x64
./build.sh windows-arm64
./build.sh android
```

`linux-arm64` on an x86_64 machine needs QEMU emulation (lasts until reboot):

```sh
docker run --privileged --rm tonistiigi/binfmt --install arm64
```

## Android

The Android project lives in `src-tauri/gen/android`. Music can be read from the internal storage or an SD card, the app asks for the audio files permission on first launch.

### Signature

Without a keystore, the APK is signed with a throwaway debug key.

```sh
keytool -genkey -v \
  -keystore ~/.android/valser-release.keystore \
  -alias valser \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000

ANDROID_KEYSTORE=~/.android/valser-release.keystore ANDROID_KEYSTORE_PASSWORD=secret ./build.sh android
```

### Install and debug
```sh
adb install -r dist/android/valser.apk
adb logcat --pid=$(adb shell pidof -s fr.magictintin.valser) 2>/dev/null
```

## CI

- `CI`: runs the tests on every push and pull request.
- `Build`: started manually from the Actions tab, builds all targets.

Android signing secrets for `Build`:

- `ANDROID_KEYSTORE_BASE64`: output of `base64 -w0 release.jks`
- `ANDROID_KEYSTORE_PASSWORD`
