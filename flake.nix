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
      buildInputs = with pkgs; [fontconfig mpv ffmpeg];
      nativeBuildInputs = with pkgs; [makeWrapper cmake pkg-config];
    in rec {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "abelscreensaver";
        version = "0.0.0";
        src = ./.;
        cargoHash = "sha256-9v622RfxvYxcSUDSZBAEwwN7zkTvRuEjgBc1hJosfQY=";
        nativeBuildInputs = nativeBuildInputs;
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
            nativeBuildInputs
            ++ buildInputs
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
