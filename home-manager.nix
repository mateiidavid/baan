{ pkgs, config, lib, ... }:
let
  cfg = config.programs.baan;
  baanPackages = pkgs.callPackage ./package.nix { };
  baan = baanPackages.baan;
in {
  options.programs.baan = {
    enable = lib.mkEnableOption "baan CLI program";

    package = lib.mkOption {
      type = lib.types.package;
      default = baan;
      description = "The baan pkg to use";
    };

    notesHomePath = lib.mkOption {
      type = lib.types.path;
      description = "Directory to store notes";
    };
  };
  config = lib.mkIf cfg.enable {

    home.sessionVariables = { };
    home.sessionVariables = { BAAN_HOME_DIR = "${cfg.notesHomePath}"; };
    home.packages = [ cfg.package ];
  };
}
