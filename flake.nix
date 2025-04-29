{
  description = "Cleanup old nix generations";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    cf.url = "github:jzbor/cornflakes";
  };

  outputs = { self, nixpkgs, cf, crane, ... }: ((cf.mkLib nixpkgs).flakeForDefaultSystems (system:
  let
    pkgs = nixpkgs.legacyPackages.${system};
    craneLib = crane.mkLib pkgs;
  in {
    packages.default = craneLib.buildPackage rec {
      src = craneLib.cleanCargoSource ./.;
      strictDeps = true;

      cargoArtifacts = craneLib.buildDepsOnly {
        inherit src strictDeps;
      };

      nativeBuildInputs = with pkgs; [
        makeWrapper
        installShellFiles
      ];
      postFixup = ''
        wrapProgram $out/bin/nix-sweep \
          --set PATH ${pkgs.lib.makeBinPath [ pkgs.nix ]}
      '';
      postInstall = ''
        mkdir ./manpages
        $out/bin/nix-sweep man ./manpages
        installManPage ./manpages/*
      '';
    };

    devShells.default = craneLib.devShell {
      inherit (self.packages.${system}.default) name;

      # Additional tools
      nativeBuildInputs = [];
    };
  })) // {
    nixosModules.default = { lib, config, pkgs, ...}:
    let
      cfg = config.services.nix-sweep;
    in {
      options.services.nix-sweep = rec {
        enable = lib.mkEnableOption "Enable nix-sweep";

        package = lib.mkOption {
          type = lib.types.package;
          inherit (self.packages.${pkgs.system}) default;
          description = "nix-sweep package to use for the service";
        };

        interval = lib.mkOption {
          type = lib.types.str;
          default = "daily";
          description = "How often to run nix-sweep (see systemd.time(7) for the format).";
        };

        keepNewer = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = 7;
          description = "Keep generations newer than <NEWER> days";
        };

        removeOlder = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = 30;
          description = "Delete generations older than <OLDER> days";
        };

        keepMin = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = 10;
          description = "Keep at least <KEEP_MIN> generations";
        };

        keepMax = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = null;
          description = "Keep at most <KEEP_MAX> generations";
        };

        gc = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Run nix garbage collection afterwards";
        };

        gcInterval = lib.mkOption {
          type = lib.types.str;
          inherit (interval) default;
          description = "How often to run garbage collection via nix-sweep (see systemd.time(7) for the format).";
        };
      };

      config = lib.mkIf cfg.enable {
        systemd = {
          timers."nix-sweep" = {
            wantedBy = [ "timers.target" ];
            timerConfig = {
              OnCalendar = cfg.interval;
              Unit = "nix-sweep.service";
            };
          };

          services."nix-sweep" = {
            script = lib.strings.concatStringsSep " " ([
              "${cfg.package}/bin/nix-sweep"
              "cleanout"
              "--non-interactive"
            ] ++ (if cfg.gc && cfg.gcInterval == cfg.interval then [ "--gc" ] else [])
              ++ (if cfg.keepMin == null then [] else [ "--keep-min" (toString cfg.keepMin) ])
              ++ (if cfg.keepMax == null then [] else [ "--keep-max" (toString cfg.keepMax) ])
              ++ (if cfg.keepNewer == null then [] else [ "--keep-newer" (toString cfg.keepNewer) ])
              ++ (if cfg.removeOlder == null then [] else [ "--remove-older" (toString cfg.removeOlder) ])
              ++ [ "system" ]
            );
            serviceConfig = {
              Type = "oneshot";
              User = "root";
            };
          };

          timers."nix-sweep-gc" = lib.mkIf (cfg.gc && cfg.gcInterval != cfg.interval) {
            wantedBy = [ "timers.target" ];
            timerConfig = {
              OnCalendar = cfg.interval;
              Unit = "nix-sweep-gc.service";
            };
          };

          services."nix-sweep-gc" = lib.mkIf (cfg.gc && cfg.gcInterval != cfg.interval) {
            script = "${cfg.package}/bin/nix-sweep gc";
            serviceConfig = {
              Type = "oneshot";
              User = "root";
            };
          };
        };
      };
    };
  };
}
