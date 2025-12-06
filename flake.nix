{
  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    # devshell.url = "github:numtide/devshell";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = {crane, ...} @ inputs:
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
        # inputs,  <-- Removed to fix the "not a perSystem module argument" error
        ...
      }: let
        # Access inputs from the lexical scope (outputs arguments)
        craneLib = inputs.crane.mkLib pkgs;

        guiInputs = (with pkgs.xorg; [libX11 libXcursor libXrandr libXi]) ++ (with pkgs; [vulkan-loader libxkbcommon wayland]);
        commonBuildInputs = with pkgs; [pkg-config freetype systemd fontconfig bluez];

        d30-cli-full = pkgs.callPackage ./pkg.nix {
          inherit craneLib;
          fullBuild = true;
          guiPreview = true;
        };
        d30-cli-preview = pkgs.callPackage ./pkg.nix {
          inherit craneLib;
          fullBuild = false;
          guiPreview = true;
        };
        d30-cli = pkgs.callPackage ./pkg.nix {
          inherit craneLib;
        };
      in {
        packages = {
          default = d30-cli-full;
          inherit d30-cli-full;
          inherit d30-cli;
          inherit d30-cli-preview;
        };
        devShells.default = pkgs.callPackage ./pkg.nix {
          inherit craneLib;
          shell = true;
        };
      };
    };
}
