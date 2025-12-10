{ pkgs ? import <nixpkgs> {} }:
let
  overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    clang
    llvmPackages.bintools
    rustup
    cargo
    rustc
    cmake
    gcc
    glib
    binutils
    gnumake
    glibc
    glibc.dev
    rustfmt
    clippy

    openssl
    pkg-config

    wayland
    wayland-protocols
    libxkbcommon
    alsa-lib
    udev
    vulkan-loader

    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
  ];

  RUSTC_VERSION = overrides.toolchain.channel;
  NIX_ENFORCE_PURITY = 0;

  LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];

  shellHook = ''
    echo "Bevy dev shell on NixOS"
    export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
    export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
    
    export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${
      pkgs.lib.makeLibraryPath [
        pkgs.xorg.libX11
        pkgs.xorg.libXcursor
        pkgs.xorg.libXi
        pkgs.xorg.libXrandr
        pkgs.libxkbcommon
        pkgs.wayland
        pkgs.vulkan-loader
        pkgs.alsa-lib
        pkgs.udev
      ]
    }

    echo "LD_LIBRARY_PATH set for Bevy runtime."

    zsh
  '';

  BINDGEN_EXTRA_CLANG_ARGS =
    (builtins.map (a: ''-I"${a}/include"'') [
      pkgs.glibc.dev
    ])
    ++ [
      ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
      ''-I"${pkgs.glib.dev}/include/glib-2.0"''
      ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
    ];
}