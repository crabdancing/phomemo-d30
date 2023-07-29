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

        baseInputs = with pkgs; [ pkg-config systemd.dev bluez.dev fontconfig.dev ];

        naersk' = pkgs.callPackage naersk {};

        mkCliBuild = pname: naersk'.buildPackage {
          inherit pname;
          src = ./.; # Adjust the source path according to your workspace layout
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = baseInputs;
          cargoBuildOptions = opts: opts ++ [ "--package" pname ];
        };

      in rec {
        defaultPackage = mkCliBuild "d30-cli";
        d30-cli = mkCliBuild "d30-cli";

        # For `nix develop`:
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ] ++ baseInputs;
          PKG_CONFIG_PATH = "${pkgs.systemd.dev}/lib/pkgconfig:${pkgs.bluez.dev}/lib/pkgconfig${pkgs.fontconfig.dev}/lib/pkgconfig";
        };
      }
    );
}
