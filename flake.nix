{
  inputs = {
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    # devshell.url = "github:numtide/devshell";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = {naersk, ...} @ inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      flake = let
        module = import ./module.nix {inherit inputs;};
      in {
        homeManagerModules = {
          default = module;
          phomemo-d30 = module;
        };
      };
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem = {
        config,
        system,
        pkgs,
        lib,
        ...
      }: let
        guiInputs = (with pkgs.xorg; [libX11 libXcursor libXrandr libXi]) ++ (with pkgs; [vulkan-loader libxkbcommon wayland]);
        commonBuildInputs = with pkgs; [pkg-config freetype systemd fontconfig bluez];

        d30-cli-full = pkgs.callPackage ./pkg.nix {
          inherit naersk;
          fullBuild = true;
          guiPreview = true;
        };
        d30-cli-preview = pkgs.callPackage ./pkg.nix {
          inherit naersk;
          fullBuild = false;
          guiPreview = true;
        };
        d30-cli = pkgs.callPackage ./pkg.nix {
          inherit naersk;
        };
      in {
        packages = {
          default = d30-cli-full;
          inherit d30-cli-full;
          inherit d30-cli;
          inherit d30-cli-preview;
        };
        devShells.default = pkgs.callPackage ./pkg.nix {
          inherit naersk;
          shell = true;
        };
      };
    };
}
