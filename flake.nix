{
  description = "Flake for building Mado packages";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devenv.url = "github:cachix/devenv";
    naersk.url = "github:nix-community/naersk";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      debug = true;
      imports = with inputs; [
        devenv.flakeModule
        treefmt-nix.flakeModule
      ];
      perSystem =
        {
          system,
          ...
        }:
        let
          pkgs = (import inputs.nixpkgs) {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };
          naersk' = pkgs.callPackage inputs.naersk {
            cargo = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
            rustc = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          };
        in
        {
          imports = [ ./flake ];
          packages = {
            default = naersk'.buildPackage { src = ./.; };
          };
        };
    };
  # flake-utils.lib.eachDefaultSystem (
  #   system:
  #   let
  #     pkgs = nixpkgs.legacyPackages.${system};
  #     os = if pkgs.stdenv.hostPlatform.isDarwin then "macOS" else "Linux-gnu";
  #     arch = if pkgs.stdenv.hostPlatform.isAarch64 then "arm64" else "x86_64";
  #
  #   in
  #   {
  #     packages = {
  #       mado = pkgs.stdenv.mkDerivation rec {
  #         pname = "mado";
  #         version = "0.3.0";
  #
  #         src = pkgs.fetchzip {
  #           stripRoot = false;
  #           url = "https://github.com/akiomik/mado/releases/download/v${version}/mado-${os}-${arch}.tar.gz";
  #           sha256 =
  #             {
  #               x86_64-linux = "10x000gza9hac6qs4pfihfbsvk6fwbnjhy7vwh6zdmwwbvf9ikis";
  #               aarch64-linux = "0qr12gib7j7h2dpxfbz02p2hfchdwkyb9xa5qlq9yyr4d3j4lvr8";
  #               x86_64-darwin = "0q33bdz2c2mjl1dn1rdy859kkkamd7m6mabsswjz0rb5cy1cyyd4";
  #               aarch64-darwin = "1cv6vqk2aq2rybhbl0ybpnrq3r2cxf03p4cd1572s8w3i4mq6rql";
  #             }
  #             .${system} or (throw "unsupported system ${system}");
  #         };
  #
  #         installPhase = ''
  #           mkdir -p $out/bin
  #           cp mado $out/bin/
  #         '';
  #
  #         meta = with pkgs.lib; {
  #           homepage = "https://github.com/akiomik/mado";
  #           description = "A fast Markdown linter written in Rust";
  #           license = licenses.asl20;
  #           sourceProvenance = [ sourceTypes.binaryNativeCode ];
  #         };
  #       };
  #       default = self.packages.${system}.mado;
  #     };
  #     formatter = pkgs.nixfmt-rfc-style;
  #   }
  # );
}
