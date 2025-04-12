let
  pkgs = import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/f9ebe33a928b.tar.gz") { };
in

pkgs.mkShellNoCC {
  packages = with pkgs; [
    cargo
    clang
    clippy
    fd
    nixfmt-rfc-style
    rustfmt
  ];
}
