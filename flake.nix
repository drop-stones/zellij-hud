{
  description = "zellij-hud: on-demand floating status bar and which-key plugin for zellij";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "wasm32-wasip1" ];
          };
        in
        {
          default = pkgs.mkShell {
            buildInputs = [
              rustToolchain
              pkgs.rust-analyzer
              pkgs.binaryen # wasm-opt
            ];
          };
        }
      );
    };
}
