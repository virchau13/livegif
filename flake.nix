{
    inputs = {
        naersk.url = "github:nix-community/naersk/master";
        nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
        utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, utils, naersk }:
        utils.lib.eachDefaultSystem (system:
            let
                pkgs = import nixpkgs { inherit system; };
                naersk-lib = pkgs.callPackage naersk { };
            in
            {
                defaultPackage = naersk-lib.buildPackage ./.;
                devShell = with pkgs; mkShell rec {
                    buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy pkg-config glib cairo libGL xorg.libX11 xorg.libxcb fontconfig.lib xorg.libXrender libglvnd xorg.libXext pixman util-linux.lib freetype xorg.libXau xorg.libXdmcp pcre2 libffi libselinux brotli.lib pcre bzip2 libpng zlib expat ];
                    shellHook = ''
                        export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"
                    '';
                    RUST_SRC_PATH = rustPlatform.rustLibSrc;
                };
            });
}
