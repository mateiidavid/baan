{
  description = "baan";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      flake-utils,
      rust-overlay,
      treefmt-nix,
      nixpkgs,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        # not legacy at all just makes nix flake check go brrr
        # https://github.com/NixOS/nixpkgs/blob/e456032addae76701eb17e6c03fc515fd78ad74f/flake.nix#L76
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolChain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
        treeConfig =
          { ... }:
          {
            programs.nixfmt-rfc-style.enable = true;
          };
        treefmtEval = treefmt-nix.lib.evalModule pkgs treeConfig;

        baanPkgs = pkgs.callPackage ./package.nix { };
      in
      {

        packages.default = baanPkgs.baan;
        packages.baan = baanPkgs.baan;
        lib.makeBaanWrapper = baanPkgs.makeBaanWrapper;

        # === DEV SHELL ===

        # https://github.com/hsjobeki/nixpkgs/blob/migrate-doc-comments/pkgs/build-support/mkshell/default.nix#L9:C1
        devShells.default =
          with pkgs;
          mkShell {
            # Concats with nativeBuildInputs
            nativeBuildInputs = [
              lld
              # cargo flame-graph
              rustToolChain
            ] ++ (lib.optional pkgs.stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
            buildInputs = [ pkg-config ];
            env = {
              # Set environment variables to help tools find rust-src
              RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/library";
              # Development
              BAAN_HOME_DIR = "./target/notes";
            };
          };

        formatter = treefmtEval.config.build.wrapper;
        checks = {
          formatting = treefmtEval.config.build.check self;

        };

      }
    )
    // (
      let
        homeManagerModule = import ./home-manager.nix;
      in
      {
        nixosModules.default = homeManagerModule;
        nixosModules.homeManagerModules.baan = homeManagerModule;
      }
    );
}
