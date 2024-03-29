{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-compat = {
      url = "github:ckiee/flake-compat/add-overrideInputs";
      flake = false;
    };
  };

  outputs = { self, flake-utils, naersk, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) { inherit system; };

        naersk' = pkgs.callPackage naersk { };

      in rec {
        # For `nix build` & `nix run`:
        packages.ledc = with pkgs;
          naersk'.buildPackage {
            src = ./ledc;
            nativeBuildInputs = [ pkg-config ];
            buildInputs = [ udev cmake fontconfig ];
            overrideMain = old: rec {
              desktopItem = makeDesktopItem rec {
                name = "ledc";
                exec = "ledc";
                desktopName = "lEdcONTROL";
                genericName = desktopName;
              };

              fixupPhase = ''
                patchelf --add-rpath ${
                  lib.makeLibraryPath [
                    stdenv.cc.cc
                    xorg.libX11
                    xorg.libXext
                    xorg.libXrender
                    xorg.libXcursor
                    xorg.libXrandr
                    xorg.libXi
                    libGL
                    fontconfig
                    freetype
                  ]
                } $out/bin/*
                install -Dm644 "${desktopItem}/share/applications/"* \
                  -t $out/share/applications/
              '';
            };
          };
        defaultPackage = packages.ledc;

        # There's a shell.nix in ./ledc for now
        # devShell =
        #  pkgs.mkShell { nativeBuildInputs = with pkgs; [ rustc cargo rust-analyzer ]; };
      });
}
