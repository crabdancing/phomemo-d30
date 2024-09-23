{inputs}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.phomemo-d30;
  tomlFormat = pkgs.formats.toml {};
  d30-cli-full = pkgs.callPackage ./pkg.nix {
    inherit (inputs) naersk;
    fullBuild = true;
    guiPreview = true;
  };
  d30-cli-minimal = pkgs.callPackage ./pkg.nix {
    inherit (inputs) naersk;
  };
in {
  options.programs.phomemo-d30 = {
    package =
      if (cfg.preview == "show_image")
      then d30-cli-full
      else d30-cli-minimal;
    enable = lib.mkEnableOption "phomemo-d30";

    default = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The default device to pick. Can be a device name, or a bluetooth address.";
      example = {
        default = "alice_desk";
      };
    };

    resolution = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = "Key-value list of device names for phomemo devices, alongside their actual addresses";
      example = {
        alice_desk = "E9:7B:61:9E:76:47";
        bob_desk = "11:94:FC:4A:99:AC";
      };
    };

    preview = lib.mkOption {
      # type = lib.types.oneOf [(lib.types.enum ["show_image" "wezterm" "gio"]) lib.types.str];
      type = lib.types.str;
      default = "gio";
      description = ''
        Preview backend to use. Defaults to `d30`.
        Requires `d30-cli-full` package for `show_image` backend.
      '';
      example = "show_image";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [
      cfg.package
    ];

    xdg.configFile."phomemo-library/phomemo-cli-config.toml".source = tomlFormat.generate "phomemo-cli-config.toml" {
      preview = cfg.preview;
    };

    xdg.configFile."phomemo-library/phomemo-config.toml".source = tomlFormat.generate "phomemo-config.toml" {
      default = cfg.default;
      resolution = cfg.resolution;
    };
  };
}
