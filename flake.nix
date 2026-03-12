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
      mkPkgs = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      mkRustToolchain = pkgs: pkgs.rust-bin.stable.latest.default.override {
        targets = [ "wasm32-wasip1" ];
      };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = mkRustToolchain pkgs;
            rustc = mkRustToolchain pkgs;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "zellij-hud";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            doCheck = false;
            buildPhase = ''
              cargo build --target wasm32-wasip1 --release
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/wasm32-wasip1/release/zellij-hud.wasm $out/bin/
            '';
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
        in
        {
          default = pkgs.mkShell {
            buildInputs = [
              (mkRustToolchain pkgs)
              pkgs.rust-analyzer
              pkgs.binaryen # wasm-opt
            ];
          };
        }
      );
    };
}
