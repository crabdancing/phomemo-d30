{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        lib = pkgs.lib;
        guiInputs = with pkgs; with pkgs.xorg; [ libX11 libXcursor libXrandr libXi vulkan-loader libxkbcommon wayland ];
        buildInputs = with pkgs; [ pkg-config systemd bluez fontconfig ];
        LD_LIBRARY_PATH = lib.makeLibraryPath (buildInputs ++ guiInputs);

        commonEnvironment = {
          inherit buildInputs;
        };

        naersk' = pkgs.callPackage naersk {};

        mkCliBuild = pname: naersk'.buildPackage (lib.recursiveUpdate commonEnvironment {
          inherit pname;
          src = ./.; # Adjust the source path according to your workspace layout
          nativeBuildInputs = with pkgs; [ pkg-config cmake makeWrapper ];
          buildInputs = buildInputs;
          cargoBuildOptions = opts: opts ++ [ "--package" pname ];
          postInstall = ''
            wrapProgram "$out/bin/d30-cli" \
              --prefix LD_LIBRARY_PATH : "${LD_LIBRARY_PATH}"
          '';
        });

      in {
        defaultPackage = mkCliBuild "d30-cli";
        d30-cli = mkCliBuild "d30-cli";

        devShell = pkgs.mkShell (lib.recursiveUpdate commonEnvironment {
          inherit LD_LIBRARY_PATH;
          shellHook = ''
            exec $SHELL
          '';
          nativeBuildInputs = with pkgs; [ rustc cargo rust-analyzer ] ++ buildInputs;
        });
      }
    );
}
