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
  rust-analyzer,
  shell ? false,
  mkShell,
  rustc,
  cargo,
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
  naerskPkg =
    naersk'.buildPackage {
      src = ./.;
      nativeBuildInputs = [pkg-config cmake makeWrapper rust-analyzer];
      inherit pname buildInputs;
      meta = {
        mainProgram = "d30-cli";
      };
    }
    // (lib.optionalAttrs guiPreview {
      postInstall = ''
        wrapProgram "$out/bin/${pname}" \
          --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (buildInputs ++ guiInputs)}"
      '';
    })
    // (lib.optionalAttrs (!fullBuild) {
      cargoBuildOptions = opts: opts ++ ["--package" pname];
    });
in
  if shell
  then
    (
      mkShell {
        nativeBuildInputs = [rustc cargo pkg-config];
        buildInputs = guiInputs ++ commonBuildInputs;
      }
    )
  else naerskPkg
