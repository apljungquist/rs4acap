{
  lib,
  rustPlatform,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ../../Cargo.lock
      ../../Cargo.toml
      ../.
    ];
  };

  cargoBuildFlags = [
    "-p"
    cargoToml.package.name
  ];
  cargoTestFlags = [
    "-p"
    cargoToml.package.name
  ];

  cargoLock.lockFile = ../../Cargo.lock;

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.package.homepage;
    license = lib.licenses.mit;
  };
}
