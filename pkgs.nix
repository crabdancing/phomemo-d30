{
  naersk,
  pkg-config,
  freetype,
  systemd,
  fontconfig,
  bluez,
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
  ...
}: let
  guiInputs = (with xorg; [libX11 libXcursor libXrandr libXi]) ++ [vulkan-loader libxkbcommon wayland];
  commonBuildInputs = [pkg-config freetype systemd fontconfig bluez];

  naersk' = callPackage naersk {};
  pname =
    if fullBuild
    then "d30-cli-full"
    else "d30-cli";
  buildInputs = commonBuildInputs ++ (lib.optionals guiPreview guiInputs);
in (naersk'.buildPackage {
    src = ./.;
    nativeBuildInputs = [pkg-config cmake makeWrapper];
    inherit pname buildInputs;
  }
  // (lib.optionalAttrs guiPreview {
    postInstall = ''
      wrapProgram "$out/bin/${pname}" \
        --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
    '';
  })
  // (lib.optionalAttrs (!fullBuild) {
    cargoBuildOptions = opts: opts ++ ["--package" pname];
  }))
# d30-cli = naersk'.buildPackage rec {
#   pname = "d30-cli";
#   src = ./.;
#   nativeBuildInputs = [pkg-config cmake makeWrapper];
#   buildInputs = commonBuildInputs;
#   cargoBuildOptions = opts: opts ++ ["--package" pname];
# };
# d30-cli-preview = naersk'.buildPackage rec {
#   pname = "d30-cli-preview";
#   src = ./.;
#   nativeBuildInputs = [pkg-config cmake makeWrapper];
#   buildInputs = commonBuildInputs ++ guiInputs;
#   cargoBuildOptions = opts: opts ++ ["--package" pname];
#   postInstall = ''
#     wrapProgram "$out/bin/${pname}" \
#       --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
#   '';
# };
# }

