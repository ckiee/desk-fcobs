{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-compat = {
      url = "github:ckiee/flake-compat/add-overrideInputs";
      flake = false;
    };
    esp-dev = {
      url = "github:thiskappaisgrey/nixpkgs-esp-dev-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, flake-utils, naersk, nixpkgs, esp-dev, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [ esp-dev.overlay ];
        };
        inherit (pkgs) lib;

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
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            # rustc
            # cargo
            clippy
            rust-analyzer
gcc-xtensa-esp32-elf-bin
rust-xtensa
          ];

          buildInputs = with pkgs; [
            udev
            cmake
            fontconfig
            openssl # cargo-generate
            openocd-esp32-bin

            # esp-idf depends
            git
            wget
            gnumake

            flex
            bison
            gperf
            pkg-config

            cmake
            ninja

            ncurses5

            python3
            python3Packages.pip
            python3Packages.virtualenv
          ];

          LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
            libGL
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            pkgs.libxml2
            pkgs.zlib
            pkgs.stdenv.cc.cc.lib
          ]);

          ESP_IDF_VERSION = "v4.4";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          ESP_IDF_SYS_ROOT_CRATE = "ledfw";
        };
      });
}
