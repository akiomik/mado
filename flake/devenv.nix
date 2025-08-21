{
  pkgs,
  config,
  ...
}:
{
  devenv.shells.default = {
    languages = {
      rust = {
        mold.enable = true;
        enable = true;
      };
      nix.enable = true;
    };
    cachix = {
      enable = true;
      pull = [ "pre-commit-hooks" ];
    };
    packages = with pkgs; [
      markdownlint-cli2
      nixd
      config.treefmt.build.wrapper
      graphviz
      statix
    ];
    git-hooks.hooks = {
      treefmt = {
        enable = true;
        package = config.treefmt.build.wrapper;
      };
    };
  };
}
