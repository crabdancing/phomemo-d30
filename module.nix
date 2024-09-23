{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.chromexup;
  tomlFormat = pkgs.formats.toml {};
  defaultPackage = pkgs.callPackage ./pkg.nix {inherit chromexup-src;};
in {
  options.programs.chromexup = {
    package = defaultPackage;
    enable = lib.mkEnableOption "chromexup";

    branding = lib.mkOption {
      type = lib.types.enum ["inox" "iridium" "chromium"];
      default = "chromium";
      description = "Name of the browser user data directory.";
    };

    parallelDownloads = lib.mkOption {
      type = lib.types.int;
      default = 4;
      description = "Parallel download threads.";
    };

    removeOrphans = lib.mkOption {
      type = lib.types.bool;
      # should do this by default to have more nixos-like behavior
      default = true;
      description = "Remove extensions not defined in the extension section.";
    };

    extensions = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = "List of browser extensions to manage.";
      example = {
        HTTPSEverywhere = "gcbommkclmclpchllfjekcdonpmejbdp";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [
      package
    ];
    xdg.configFile."chromexup/config.ini".source = tomlFormat.generate "config.ini" {
      main = {
        branding = cfg.branding;
        parallel_downloads = toString cfg.parallelDownloads;
        remove_orphans =
          if cfg.removeOrphans
          then "True"
          else "False";
      };
      extensions = cfg.extensions;
    };

    # systemd.user.timers.chromexup = {
    #   Unit = {
    #     Description = "Run chromexup daily";
    #   };

    #   Timer = {
    #     OnActiveSec = 10;
    #     OnCalendar = "daily";
    #     Persistent = true;
    #   };

    #   Install = {
    #     WantedBy = ["timers.target"];
    #   };
    # };

    # systemd.user.services.chromexup = {
    #   Unit = {
    #     Description = "External extension updater for Chromium based browsers";
    #     After = ["network-online.target" "psd-resync.service"];
    #     Wants = ["network-online.target"];
    #   };

    #   Service = {
    #     Type = "simple";
    #     ExecStart = "${package}/bin/chromexup";
    #   };
    # };
  };
}
