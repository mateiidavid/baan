{ pkgs, ... }:
  pkgs.rustPlatform.buildRustPackage {
    pname = "baan";
    version = "0.1.1";

    src = ./.;

    # Computed from the crate sources; i.e. all deps are hashed and
    # that gives a checksum. obtained w/ a fake checksum
    cargoHash = "sha256-+P/yYRLBCv4UE7vw1Wp7kLcy0bKyTFUFa42nogvt10g=";

    meta = with pkgs.lib; {
      description = "A CLI to help me takes notes using Helix";
      homepage = "https://github.com/mateiidavid/baan";
      license = licenses.mit;
      maintainers = [ maintainers.mateiidavid ];
      platforms = platforms.all;
    };
}
