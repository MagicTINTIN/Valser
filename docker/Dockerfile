# syntax=docker/dockerfile:1

FROM ubuntu:24.04 AS base
ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential ca-certificates cmake curl file pkg-config libwebkit2gtk-4.1-dev libdbus-1-dev libasound2-dev \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
RUN cargo install tauri-cli --version "^2" --locked
WORKDIR /src

FROM base AS linux-build
ARG TARGETARCH
ENV APPIMAGE_EXTRACT_AND_RUN=1
COPY . .
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=linux-target-$TARGETARCH,target=/src/target \
    cargo tauri build --bundles deb,appimage \
    && mkdir /out \
    && cp target/release/valser /out/valser \
    && cp target/release/bundle/deb/*.deb target/release/bundle/appimage/*.AppImage /out/

FROM scratch AS linux
COPY --from=linux-build /out /

FROM base AS windows-build
ARG ARCH
RUN apt-get update && apt-get install -y --no-install-recommends nsis lld llvm clang ninja-build \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add $ARCH-pc-windows-msvc && cargo install cargo-xwin --locked
COPY . .
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=windows-target,target=/src/target \
    --mount=type=cache,id=xwin,target=/root/.cache/cargo-xwin \
    cargo tauri build --runner cargo-xwin --target $ARCH-pc-windows-msvc --bundles nsis \
    && mkdir /out \
    && cp target/$ARCH-pc-windows-msvc/release/valser.exe /out/valser.exe \
    && cp target/$ARCH-pc-windows-msvc/release/bundle/nsis/*.exe /out/

FROM scratch AS windows
COPY --from=windows-build /out /

FROM base AS android-base
ENV ANDROID_HOME=/opt/android \
    NDK_HOME=/opt/android/ndk/27.2.12479018 \
    ANDROID_NDK_ROOT=/opt/android/ndk/27.2.12479018
RUN apt-get update && apt-get install -y --no-install-recommends openjdk-17-jdk-headless unzip \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSfo /tmp/tools.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip \
    && unzip -q /tmp/tools.zip -d /tmp/tools \
    && mkdir -p $ANDROID_HOME/cmdline-tools \
    && mv /tmp/tools/cmdline-tools $ANDROID_HOME/cmdline-tools/latest \
    && rm -rf /tmp/tools /tmp/tools.zip \
    && yes | $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager --licenses > /dev/null \
    && $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager "platform-tools" "platforms;android-36" "platforms;android-37.0" "build-tools;36.0.0" "ndk;27.2.12479018"
RUN rustup target add aarch64-linux-android x86_64-linux-android

FROM android-base AS android-build
COPY . .
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=android-target,target=/src/target \
    --mount=type=cache,id=gradle,target=/root/.gradle \
    --mount=type=secret,id=keystore \
    --mount=type=secret,id=keystore_password,env=KEYSTORE_PASSWORD \
    cargo tauri icon src-tauri/icons/icon.png \
    && cargo tauri android build --apk --target aarch64 --target x86_64 \
    && mkdir /out \
    && docker/sign-apk.sh

FROM scratch AS android
COPY --from=android-build /out /
