{
  pkgs ? import <nixpkgs> { },
  lib ? pkgs.lib,
}:
let
  packages = with pkgs; [
    rust-analyzer
    rustfmt
    mold
    rust-bin.stable.latest.default
    cmake

    pkg-config
    xorg.libX11
    libGL
    alsa-lib
    xorg.libXi
    pkgs.libGL

    # X11 dependencies
    xorg.libX11
    xorg.libX11.dev
    xorg.libXcursor
    xorg.libXi
    xorg.libXinerama
    xorg.libXrandr
    emscripten # web support
    libxkbcommon

    pkg-config

    # (rust-bin.stable.latest.default.override {
    #   targets = [ "wasm32-unknown-unknown" ];
    # })
  ];
in
pkgs.mkShell {
  # Get dependencies from the main package
  inputsFrom = [ (pkgs.callPackage ./default.nix { }) ];
  nativeBuildInputs = packages;
  buildInputs = packages;
  env = {
    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    LD_LIBRARY_PATH = "${lib.makeLibraryPath packages}";
  };
}
