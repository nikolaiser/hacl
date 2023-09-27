{
  description = "Simple home assistant cli";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs:
    inputs.flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
          };

          haclPackage = inputs.nixpkgs.legacyPackages.${system}.callPackage ./. { };

        in
        {
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs;
              [ pkg-config openssl ];
          };

          packages = rec {
            hacl = haclPackage;
            default = hacl;
          };

          nixosModules.hm = { config, ... }:
            let
              cfg = config.programs.hacl;
            in
            {
              options.programs.hacl = {
                enable = pkgs.lib.options.mkEnableOption "Hacl";
              };

              config = pkgs.lib.mkIf cfg.enable {
                home.packages = [ haclPackage ];
              };
            };
        }
      );
}
