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
    in {
      devShell = with pkgs;
        mkShell {
          buildInputs = [
            cargo
            rust-analyzer
            rustfmt
            clippy
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath [wayland libxkbcommon libGL]}";
        };
    });
}
