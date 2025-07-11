{ pkgs, ... }:
let
  baanBase = pkgs.rustPlatform.buildRustPackage {
    pname = "baan";
    version = "0.1.0";

    src = ./.;

    # Computed from the crate sources; i.e. all deps are hashed and
    # that gives a checksum. obtained w/ a fake checksum
    cargoHash = "sha256-d1beS0rUpRxE+TQZp5U6LEbuv+8WMvhDg5nPP+w05J0=";

    meta = with pkgs.lib; {
      description = "A CLI to help me takes notes using Helix";
      homepage = "https://github.com/mateiidavid/baan";
      license = licenses.mit;
      maintainers = [ maintainers.mateiidavid ];
      platforms = platforms.all;
    };
  };
in
{

  # Base baan package
  baan = baanBase;

  # Create a configured wrapper for simple uses cases (i.e. outside of home-manager)
  makeBaanWrapper =
    {
      homeDirPath ? "${builtins.getEnv "HOME"}/notes",
    }:
    pkgs.writeShellScriptBin "baan" ''
      export BAAN_HOME_DIR="${homeDirPath}"
      exec ${baanBase}/bin/baan "$@"
    '';
}
