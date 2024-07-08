{
  description = "abelscreensaver";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in rec {
      abelscreensaver = pkgs.rustPlatform.buildRustPackage {
        pname = "abelscreensaver";
        version = "0.0.0";
        src = ./.;
        cargoHash = "sha256-lK7g0j0cObSP1/rVM1jd402OJAFHo6psOutsSDkK33w=";
      };
      apps.default = {
        type = "app";
        program = "${abelscreensaver}/bin/abelscreensaver";
      };
      devShell = with pkgs;
        mkShell {
          buildInputs = [
            cargo
            rustc
            rust-analyzer
            rustfmt
            clippy
            mpv
            cmake
            pkg-config
            fontconfig
          ];
        };
    });
}
