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
      buildInputs = with pkgs; [mpv ffmpeg];
    in rec {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "abelscreensaver";
        version = "0.0.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        cargoLock.outputHashes = {
          "egui-0.28.1" = "sha256-/2aiBKv85XW+dnn+T45e5UxTH0nyhKcHYHfklFSTu7U=";
        };
        nativeBuildInputs = [pkgs.makeWrapper];
        buildInputs = buildInputs;
        postInstall = ''
          wrapProgram $out/bin/abelscreensaver \
            --prefix PATH : ${pkgs.lib.makeBinPath [pkgs.ffmpeg]}
        '';
      };
      apps.default = {
        type = "app";
        program = "${packages.default}/bin/abelscreensaver";
      };
      devShell = with pkgs;
        mkShell {
          nativeBuildInputs =
            buildInputs
            ++ [
              cargo
              rustc
              rust-analyzer
              rustfmt
              clippy
            ];
        };
    });
}
