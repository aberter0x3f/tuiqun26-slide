{
  description = "Reproducible Typst slide development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              typst
              just
              (python3.withPackages (pythonPackages: [
                pythonPackages.graphviz
                pythonPackages.matplotlib
              ]))
              graphviz
              poppler-utils
              ghostscript
              fontconfig
              newcomputermodern
              source-han-serif
              sarasa-gothic
            ];
          };
        }
      );
    };
}
