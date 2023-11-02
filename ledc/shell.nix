{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell {
  buildInputs = [ udev pkg-config cmake fontconfig rust-analyzer rustc cargo clippy rustfmt ];
  LD_LIBRARY_PATH =
    lib.makeLibraryPath [
      libGL
      libxkbcommon
      wayland
      xorg.libX11
      xorg.libXcursor
      xorg.libXi
      xorg.libXrandr
    ];
}
