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
        module = ./module.nix;
      in {
        nixosModules = {
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
        # guiInputs = (with pkgs.xorg; [libX11 libXcursor libXrandr libXi]) ++ (with pkgs; [vulkan-loader libxkbcommon wayland]);
        # commonBuildInputs = with pkgs; [pkg-config freetype systemd fontconfig bluez];
        # naersk' = pkgs.callPackage naersk {};
        d30-cli-full = pkgs.callPackage ./pkgs.nix {
          inherit naersk;
          fullBuild = true;
          guiPreview = true;
        };
        d30-cli-preview = pkgs.callPackage ./pkgs.nix {
          inherit naersk;
          fullBuild = false;
          guiPreview = true;
        };
        d30-cli = pkgs.callPackage ./pkgs.nix {
          inherit naersk;
        };
        # d30-cli-full = naersk'.buildPackage rec {
        #   pname = "d30-cli";
        #   src = ./.;
        #   nativeBuildInputs = with pkgs; [pkg-config cmake makeWrapper];
        #   buildInputs = commonBuildInputs ++ guiInputs;
        #   postInstall = ''
        #     wrapProgram "$out/bin/${pname}" \
        #       --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
        #   '';
        # };
        # d30-cli = naersk'.buildPackage rec {
        #   pname = "d30-cli";
        #   src = ./.;
        #   nativeBuildInputs = with pkgs; [pkg-config cmake makeWrapper];
        #   buildInputs = commonBuildInputs;
        #   cargoBuildOptions = opts: opts ++ ["--package" pname];
        # };
        # d30-cli-preview = naersk'.buildPackage rec {
        #   pname = "d30-cli-preview";
        #   src = ./.;
        #   nativeBuildInputs = with pkgs; [pkg-config cmake makeWrapper];
        #   buildInputs = commonBuildInputs ++ guiInputs;
        #   cargoBuildOptions = opts: opts ++ ["--package" pname];
        #   postInstall = ''
        #     wrapProgram "$out/bin/${pname}" \
        #       --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
        #   '';
        # };
        # shell = pkgs.mkShell {
        #   LD_LIBRARY_PATH = lib.makeLibraryPath (commonBuildInputs ++ guiInputs);
        #   shellHook = ''
        #     exec $SHELL
        #   '';
        #   nativeBuildInputs = with pkgs; [rustc cargo rust-analyzer] ++ commonBuildInputs;
        # };
      in {
        # imports = [
        #   inputs.devshell.flakeModule
        # ];

        packages = {
          default = d30-cli-full;
          inherit d30-cli-full;
          inherit d30-cli;
          inherit d30-cli-preview;
        };

        # devshells = {
        #   default = shell;
        #   d30-cli = shell;
        # };
      };
    };
}
