{
  lib,
  rustPlatform,
  dir,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./crates/${dir}/Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  inherit (cargoToml.package) version;
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.lock
      ./Cargo.toml
      ./crates
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  cargoBuildFlags = [
    "-p"
    cargoToml.package.name
  ];
  cargoTestFlags = [
    "-p"
    cargoToml.package.name
  ];
}
