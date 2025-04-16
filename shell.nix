let
  pkgs = import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/f9ebe33a928b.tar.gz") { };
  mkhelp = pkgs.callPackage ./pkgs/mkhelp { };
in

pkgs.mkShellNoCC {
  packages = with pkgs; [
    avahi
    cargo
    clang
    clippy
    fd
    git
    llvmPackages.libclang
    mkhelp
    nixfmt-rfc-style
    rustc
    rustfmt
  ];

  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
}
