{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      naersk,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        formatter = pkgs.nixfmt-tree;

        packages = {
          parkour-api = naersk-lib.buildPackage ./.;
          default = self.packages.${system}.parkour-api;
        };
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              cargo
              rustc
              rustfmt
              pre-commit
              rustPackages.clippy
              rust-analyzer
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };

        nixosModules.default = (
          {
            config,
            lib,
            pkgs,
            ...
          }:
          let
            cfg = config.services.parkour-api;
          in
          {
            options.services.parkour-api = {
              enable = lib.mkEnableOption "Parkour API server";

              openFirewall = lib.mkEnableOption "Open port 3031 on the firewall";

              apiKey = lib.mkOption {
                description = "Specifies the api key used by parkour-api.";
                type = lib.types.str;
                default = null;
              };

              apiKeyFile = lib.mkOption {
                description = "Specifies the api key used by parkour-api using a file.";
                type = lib.types.nullOr (lib.types.pathWith { absolute = true; });
                example = "/run/secrets/api-key";
                default = null;
              };

              package = lib.mkPackageOption self.packages.${pkgs.stdenv.hostPlatform.system} "parkour-api" { };
            };

            config = lib.mkIf cfg.enable {
              users.users.parkour-api = {
                isSystemUser = true;
                group = "parkour-api";
              };

              users.groups.parkour-api = { };

              systemd.services.parkour-api = {
                description = "Parkour API server";
                wantedBy = [ "multi-user.target" ];
                after = [
                  "network.target"
                ];

                preStart = ''
                  export PATH=${lib.makeBinPath [ pkgs.coreutils ]}
                  install -d -m 0750 /var/lib/parkour-api/scoreboard
                  cp -f ${./scoreboard/template.html} /var/lib/parkour-api/scoreboard/template.html
                '';

                serviceConfig = {
                  Restart = "always";
                  KillSignal = "SIGINT";
                  User = "parkour-api";
                  Group = "parkour-api";
                  WorkingDirectory = "/var/lib/parkour-api";
                  StateDirectory = "parkour-api";
                  UMask = "0022";
                  EnvironmentFile =
                    if cfg.apiKeyFile == null then
                      pkgs.writeText "apikey" ''
                        PARKOUR_API_SECRET=${cfg.apiKey}
                      ''
                    else
                      cfg.apiKeyFile;

                  ExecStart = "${cfg.package}/bin/parkour-api";

                  # Sandboxing
                  NoNewPrivileges = true;
                  PrivateDevices = true;
                  ProtectSystem = "full";
                  ReadWritePaths = [ "/var/lib/parkour-api" ];
                  ProtectHome = true;
                  ProtectControlGroups = true;
                  ProtectKernelModules = true;
                  ProtectKernelTunables = true;
                  RestrictRealtime = true;
                  RestrictNamespaces = true;
                  MemoryDenyWriteExecute = true;
                };
              };

              systemd.tmpfiles.rules = lib.optional (
                cfg.apiKeyFile == null
              ) "f /var/lib/parkour-api/env 0640 - - - PARKOUR_API_SECRET=${cfg.apiKey}";

              networking.firewall.allowedTCPPorts = lib.optional cfg.openFirewall 3031;
            };
          }
        );
      }
    );
}
