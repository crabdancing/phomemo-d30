{
  craneLib,
  pkg-config,
  freetype,
  systemd,
  fontconfig,
  bluez,
  expat,
  xorg,
  vulkan-loader,
  libxkbcommon,
  wayland,
  callPackage,
  cmake,
  makeWrapper,
  lib,
  guiPreview ? false,
  fullBuild ? false,
  rust-analyzer,
  shell ? false,
  mkShell,
  rustc,
  cargo,
  ...
}: let
  guiInputs = (with xorg; [libX11 libXcursor libXrandr libXi]) ++ [vulkan-loader libxkbcommon wayland];
  commonBuildInputs = [pkg-config freetype systemd fontconfig bluez expat];

  pname =
    if fullBuild
    then "d30-cli-full"
    else "d30-cli";

  # The binary name typically matches the crate name, not strictly the pname
  binaryName = "d30-cli";

  buildInputs = commonBuildInputs ++ (lib.optionals guiPreview guiInputs);

  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type:
      (craneLib.filterCargoSources path type) || (lib.hasSuffix ".ttf" path);
  };

  cranePkg = craneLib.buildPackage ({
      inherit src;
      nativeBuildInputs = [pkg-config cmake makeWrapper rust-analyzer];
      inherit pname buildInputs;
      meta = {
        mainProgram = binaryName;
      };
    }
    // (lib.optionalAttrs guiPreview {
      postInstall = ''
        wrapProgram "$out/bin/${binaryName}" \
          --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
      '';
    })
    // (lib.optionalAttrs (!fullBuild) {
      # When not fullBuild, we build the specific package "d30-cli"
      cargoExtraArgs = "--package d30-cli";
    }));
in
  if shell
  then
    (
      mkShell {
        nativeBuildInputs = [rustc cargo pkg-config];
        buildInputs = guiInputs ++ commonBuildInputs;
      }
    )
  else cranePkg
