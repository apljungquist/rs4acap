{
  lib,
  fetchFromGitHub,
  rustPlatform,
}:

rustPlatform.buildRustPackage rec {
  pname = "mkhelp";
  version = "0.2.3";

  src = fetchFromGitHub {
    owner = "apljungquist";
    repo = "mkhelp-rs";
    rev = "v${version}";
    hash = "sha256-Rj+bhopZLQr/bIkwNlaRji/aOm3qWce/82qmLrQNwZU=";
  };

  useFetchCargoVendor = true;
  cargoHash = "sha256-wcF5e8RfPNMgXyfhtJO+spXdnW+DvYMnwlk/0O+mmZY=";

  meta = {
    description = "Support for docstrings in makefiles";
    homepage = "https://github.com/apljungquist/mkhelp-rs";
    license = lib.licenses.mit;
    maintainers = [ ];
  };
}
