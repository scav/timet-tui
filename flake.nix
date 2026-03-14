{
  description = "Simple TUI app for Timet";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          toolchain = fenix.packages.${system}.minimal.toolchain;

          timet-tui =
            (pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            }).buildRustPackage
              {

                pname = "timet-tui";
                version = "0.5.0";
                src = ./.;
                cargoLock.lockFile = ./Cargo.lock;
                nativeBuildInputs = [
                  pkgs.git
                  toolchain
                ];

                doCheck = true;
                strip = true;
              };
        in
        {
          default = timet-tui;

          docker = pkgs.dockerTools.buildLayeredImage {
            name = "timet-tui";
            tag = "latest";

            contents = [
              timet-tui
              pkgs.cacert
              pkgs.iana-etc
            ];

            config = {
              Cmd = [ "timet-tui" ];
              Env = [
                "TERM=xterm-256color"
                "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
              ];
            };
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          toolchain = fenix.packages.${system}.combine [
            fenix.packages.${system}.stable.toolchain
            fenix.packages.${system}.stable.rust-src
          ];
        in
        {
          default = pkgs.mkShellNoCC {
            buildInputs = [ toolchain ];
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/timet-tui";
        };
      });

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.alejandra);
    };
}
