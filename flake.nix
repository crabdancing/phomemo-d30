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
        guiInputs = with pkgs; with pkgs.xorg; [ libX11 libXcursor libXrandr libXi vulkan-loader libxkbcommon wayland fontconfig ];
        backendInputs = with pkgs; [ systemd bluez ];
        commonBuildInputs = with pkgs; [ pkg-config ];

        naersk' = pkgs.callPackage naersk {};
        
        d30-cli = naersk'.buildPackage rec {
          pname = "d30-cli";
          src = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config cmake makeWrapper ];
          buildInputs = commonBuildInputs ++ backendInputs;
          cargoBuildOptions = opts: opts ++ [ "--package" pname ];
        };
        
        d30-cli-preview = naersk'.buildPackage rec {
          pname = "d30-cli-preview";
          src = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config cmake makeWrapper ];
          buildInputs = commonBuildInputs ++ guiInputs;
          cargoBuildOptions = opts: opts ++ [ "--package" pname ];
          postInstall = ''
            wrapProgram "$out/bin/${pname}" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
          '';
        };

      in {
        defaultPackage = d30-cli;
        inherit d30-cli;
        inherit d30-cli-preview;

        devShell = pkgs.mkShell {
          LD_LIBRARY_PATH = lib.makeLibraryPath (commonBuildInputs ++ guiInputs ++ backendInputs);
          shellHook = ''
            exec $SHELL
          '';
          nativeBuildInputs = with pkgs; [ rustc cargo rust-analyzer ] ++ buildInputs;
        };
      }
    );
}
