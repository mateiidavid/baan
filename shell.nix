# Ref:
# https://github.com/hsjobeki/nixpkgs/blob/migrate-doc-comments/pkgs/build-support/mkshell/default.nix#L9:C1
{
  darwin,
  lib,
  lld,
  mkShell,
  pkg-config,
  rust-bin,
  stdenv,
}:
let
  rustToolChain = rust-bin.nightly.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
    ];
  };
in
mkShell {
  # Concats with nativeBuildInputs
  nativeBuildInputs = [
    lld
    # cargo flame-graph
    rustToolChain
  ] ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
  buildInputs = [ pkg-config ];
  env = {
    # Set environment variables to help tools find rust-src
    RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/library";
    # Development
    BAAN_LOG_LEVEL="baan=info";
    BAAN_LOCAL_DEV=1;
  };
}
