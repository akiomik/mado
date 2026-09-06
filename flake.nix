{
  description = "Flake for building Mado packages";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        os = if pkgs.stdenv.hostPlatform.isDarwin then "macOS" else "Linux-gnu";
        arch = if pkgs.stdenv.hostPlatform.isAarch64 then "arm64" else "x86_64";

      in
      {
        packages = {
          mado = pkgs.stdenv.mkDerivation rec {
            pname = "mado";
            version = "0.3.2";

            src = pkgs.fetchzip {
              stripRoot = false;
              url = "https://github.com/akiomik/mado/releases/download/v${version}/mado-${os}-${arch}.tar.gz";
              sha256 =
                {
                  x86_64-linux = "01pr100kq24638hil6vm6s6dmjv26ljbaibbzg3jik85vpagh3mb";
                  aarch64-linux = "0wbanyvgs744mb6wp1jcyhz9bpr1f9f9k6ai7izs8q5cn8sq7z6z";
                  x86_64-darwin = "1rq4axj2pxzw39bc6njfx7hpmy1gg89igj96f4znwjj7nfd5mxjs";
                  aarch64-darwin = "0wrinjdh5jy9a0nk0f2y8faflwdlrvz930vc1xwsyjivy4j27sgl";
                }
                .${system} or (throw "unsupported system ${system}");
            };

            installPhase = ''
              mkdir -p $out/bin
              cp mado $out/bin/
            '';

            meta = with pkgs.lib; {
              homepage = "https://github.com/akiomik/mado";
              description = "A fast Markdown linter written in Rust";
              license = licenses.asl20;
              sourceProvenance = [ sourceTypes.binaryNativeCode ];
            };
          };
          default = self.packages.${system}.mado;
        };
        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
