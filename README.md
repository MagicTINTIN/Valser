# Valser
Rust Music player

## Dev

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
```

### Build
```sh
cargo run -p valser 
```

## Android

### Signature

```sh
keytool -genkey -v \
  -keystore ~/.android/valser-release.keystore \
  -alias valser \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000

# then update the valser-android/Cargo.toml credentials
```

### Push and update
```sh
# build
cargo apk build -p valser-android --release

# install
adb install target/release/apk/valser_android.apk

# debug && read logs
adb logcat --pid=$(adb shell pidof -s fr.magictintin.valser) 2>/dev/null
```