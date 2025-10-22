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
        baanPkg = pkgs.callPackage ./package.nix {};
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
        packages.default = baanPkg;
      } // {
        overlays = {
            default = prev: final: { baan = self.packages.${prev.system}.baan; };
        };
      } 
    );
}
