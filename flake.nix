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
                  "systemd-tmpfiles-setup.service"
                ];
                requires = [ "systemd-tmpfiles-setup.service" ];

                preStart = ''
                  export PATH=${lib.makeBinPath [ pkgs.coreutils ]}
                  rm -rf /var/lib/parkour-api/scoreboard
                  rm -rf /var/lib/parkour-api/admin
                  ln -s ${./scoreboard} /var/lib/parkour-api/scoreboard
                  ln -s ${./admin} /var/lib/parkour-api/admin
                '';

                serviceConfig = {
                  Restart = "always";
                  KillSignal = "SIGINT";
                  User = "parkour-api";
                  Group = "parkour-api";
                  WorkingDirectory = "/var/lib/parkour-api";
                  StateDirectory = "parkour-api";
                  UMask = "0022";
                  EnvironmentFile = "/var/lib/parkour-api/env";
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

              systemd.tmpfiles.rules = [
                "d /var/lib/parkour-api 0640 parkour parkour -"
                "d /var/lib/parkour-api/data 0640 parkour parkour -"
              ]
              ++ lib.optional (
                cfg.apiKeyFile == null
              ) "f /var/lib/parkour-api/env 0640 parkour parkour - PARKOUR_API_SECRET=${cfg.apiKey}"
              ++ lib.optional (
                cfg.apiKeyFile != null
              ) "c /var/lib/parkour-api/env 0640 parkour parkour - ${cfg.apiKeyFile}";

              networking.firewall.allowedTCPPorts = lib.optional cfg.openFirewall 3031;
            };
          }
        );
      }
    );
}
