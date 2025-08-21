{
  treefmt.programs = {
    nixfmt.enable = true;
    statix.enable = true;
    taplo = {
      enable = true;
      settings = {
        exclude = [ ".git/**/*" ];
        rule = [
          {
            include = [ "**/mado.toml" ];
            schema = {
              path = "./pkg/json-schema/mado.json";
            };
          }
        ];
      };
    };
    typos = {
      enable = true;
      configFile = "typos.toml";
    };
  };
}
