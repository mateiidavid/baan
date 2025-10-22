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
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        treeConfig =
          { ... }:
          {
            programs.nixfmt.enable = true;
          };
        treefmtEval = treefmt-nix.lib.evalModule pkgs treeConfig;
        baanPkgs = pkgs.callPackage ./package.nix { };
      in
      {

        checks = {
          formatting = treefmtEval.config.build.check self;
        };
        devShells.default = import ./shell.nix {
          inherit (pkgs)
            darwin
            lib
            lld
            mkShell
            pkg-config
            rust-bin
            stdenv
            ;
        };
        formatter = treefmtEval.config.build.wrapper;
        packages.default = baanPkgs.baan;
        packages.baan = baanPkgs.baan;
      }
    )
    // {
      overlays.default = final: prev: {
        baan = (prev.callPackage ./package.nix { }).baan;
      };
    };
}
