# Downloads and extracts the ACAP Native SDK(s) into the Nix store.
#
# The SDK is only distributed as a set of Docker images, one per target
# architecture. We pull the images with `dockerTools.pullImage`, flatten their
# layers, and keep only the contents of `/opt/axis`. Both architectures are
# merged into a single tree to support targetting both target architectures with
# a single environment variable
#
# To bump the SDK version, update `version` and the two `imageDigest`s (find the
# digest with `docker manifest inspect --verbose <image>:<tag>`), set the
# corresponding `sha256` to `lib.fakeHash`, run `nix build .#acap-native-sdk`,
# and copy the hash Nix reports back into this file.
#
# Keep the version in sync with:
# - .devcontainer/acap-native-sdk-12-aarch64/devcontainer.json
# - .devcontainer/acap-native-sdk-12-armv7hf/devcontainer.json
# - .github/workflows/fuzz.yaml
# - .github/workflows/main.yaml
# - bin/create-venv.sh
{
  lib,
  stdenvNoCC,
  dockerTools,
  jq,
}:
let
  version = "12.11.0";
  ubuntuTag = "ubuntu24.04";

  # The SDK images are published for amd64 hosts only; the `armv7hf`/`aarch64`
  # in the tag refers to the *target* architecture, not the host.
  pullSdk =
    {
      targetArch,
      imageDigest,
      sha256,
    }:
    dockerTools.pullImage {
      imageName = "axisecp/acap-native-sdk";
      finalImageName = "axisecp/acap-native-sdk";
      finalImageTag = "${version}-${targetArch}-${ubuntuTag}";
      inherit imageDigest sha256;
      os = "linux";
      arch = "amd64";
    };

  images = [
    (pullSdk {
      targetArch = "armv7hf";
      imageDigest = "sha256:355a328268d0184bc7085646fee5c9bdab7e2dd0fe8952029828a68392fc3b61";
      sha256 = "sha256-MDclOr/sJIJ1GFgo5fT9dfVII6fMvH+OMWGjReeemF4=";
    })
    (pullSdk {
      targetArch = "aarch64";
      imageDigest = "sha256:44c54f65c5a1475020274520582b7a22d88505c61902c369c97bd81db98725db";
      sha256 = "sha256-21bt/SixxmOVzfpGp2ooyDKfa7+Lxz72ZFfwclEhyOc=";
    })
  ];
in
stdenvNoCC.mkDerivation {
  pname = "acap-native-sdk";
  inherit version;

  dontUnpack = true;

  # Keep the SDK byte-for-byte as shipped: no RPATH shrinking, shebang
  # patching, stripping, or symlink checks. The sysroots contain dangling
  # symlinks by design and patching their ELF files would corrupt the
  # cross-compilation toolchain.
  dontFixup = true;

  nativeBuildInputs = [ jq ];

  # Flatten each image and keep `/opt/axis`. Layers are applied in order so that
  # later layers win, matching how a container sees the filesystem. Each layer
  # is streamed straight out of the image tarball to avoid materialising every
  # layer on disk at once.
  buildPhase = ''
    runHook preBuild

    mkdir -p "$out"
    work="$(mktemp -d)"
    for image in ${lib.escapeShellArgs images}; do
      tar -xf "$image" -C "$work" manifest.json
      for layer in $(jq -r '.[0].Layers[]' "$work/manifest.json"); do
        # The SDK ships read-only directories, and the build runs as an
        # unprivileged user that cannot bypass directory permissions. Make every
        # directory extracted so far writable so the next layer can create
        # entries inside them.
        find "$out" -type d ! -perm -u+w -exec chmod u+w {} + 2>/dev/null || true
        # `--strip-components 2` drops the leading `opt/axis/`, so the SDK
        # contents end up directly under `$out` (as in the Makefile). Layers
        # that don't touch `/opt/axis` make the inner tar exit non-zero, which
        # is expected and ignored.
        tar -xOf "$image" "$layer" \
          | tar -x -C "$out" \
              --strip-components 2 \
              --no-same-owner \
              'opt/axis' 2>/dev/null || true
      done
    done
    rm -rf "$work"

    # Drop any stray OverlayFS whiteout markers picked up while flattening.
    find "$out" -name '.wh.*' -delete

    runHook postBuild
  '';

  dontInstall = true;

  meta = {
    description = "ACAP Native SDK, extracted from the official Docker images";
    homepage = "https://github.com/AxisCommunications/acap-native-sdk";
    platforms = lib.platforms.all;
  };
}
