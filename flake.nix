{
  description = "Simple TUI app for Timet";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in
    {

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          toolchain = fenix.packages.${system}.stable.toolchain;
        in
        {
          default = pkgs.mkShell {
            buildInputs = [ toolchain ];
          };
        }
      );

      checks = forAllSystems (system: {
        default = self.packages.${system}.default;
      });

      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          toolchain = fenix.packages.${system}.stable.toolchain;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "timet-tui";
            version = "0.5.0";
            src = ./.;
            cargoHash = "sha256-ZywKTmhIKlr9N7yaSP2nTdT9M5yI6yOBdqxlxxKrAdA=";
            nativeBuildInputs = [ pkgs.git ];
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/timet-tui";
        };
      });
    };
}
