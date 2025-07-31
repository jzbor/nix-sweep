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
  })) // (let
    mkOptions = { lib, pkgs, defaultProfiles }: rec {
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

      profiles = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = defaultProfiles;
        description = "What profiles to run nix-sweep on.";
      };

      keepNewer = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = "7d";
        description = "Keep generations newer than <NEWER> days.";
      };

      removeOlder = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = "30d";
        description = "Delete generations older than <OLDER> days.";
      };

      keepMin = lib.mkOption {
        type = lib.types.nullOr lib.types.int;
        default = 10;
        description = "Keep at least <KEEP_MIN> generations.";
      };

      keepMax = lib.mkOption {
        type = lib.types.nullOr lib.types.int;
        default = null;
        description = "Keep at most <KEEP_MAX> generations.";
      };

      gc = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Run Nix garbage collection afterwards.";
      };

      gcBigger = lib.mkOption {
        type = lib.types.nullOr lib.types.int;
        default = null;
        description = "Only perform gc if store is bigger than this many GiB";
      };

      gcQuota = lib.mkOption {
        type = lib.types.nullOr lib.types.int;
        default = null;
        description = "Only perform gc if store uses more than this many % of its device";
      };

      gcInterval = lib.mkOption {
        type = lib.types.str;
        inherit (interval) default;
        description = "How often to run garbage collection via nix-sweep (see systemd.time(7) for the format).";
      };
    };

    mkServiceScripts = { lib, cfg }: {
      "nix-sweep" = lib.strings.concatStringsSep " " ([
        "${cfg.package}/bin/nix-sweep"
        "cleanout"
        "--non-interactive"
      ] ++ (if cfg.gc && cfg.gcInterval == cfg.interval then [ "--gc" ] else [])
        ++ (if cfg.gcBigger == null then [] else [ "--gc-bigger" (toString cfg.gcBigger) ])
        ++ (if cfg.gcQuota == null then [] else [ "--gc-quota" (toString cfg.gcQuota) ])
        ++ (if cfg.keepMin == null then [] else [ "--keep-min" (toString cfg.keepMin) ])
        ++ (if cfg.keepMax == null then [] else [ "--keep-max" (toString cfg.keepMax) ])
        ++ (if cfg.keepNewer == null then [] else [ "--keep-newer" cfg.keepNewer ])
        ++ (if cfg.removeOlder == null then [] else [ "--remove-older" cfg.removeOlder ])
        ++ cfg.profiles
      );

      "nix-sweep-gc" = lib.strings.concatStringsSep " " ([
        "${cfg.package}/bin/nix-sweep"
        "gc"
        "--non-interactive"
      ] ++ (if cfg.gcBigger == null then [] else [ "--bigger" (toString cfg.gcBigger) ])
        ++ (if cfg.gcQuota == null then [] else [ "--quota" (toString cfg.gcQuota) ])
      );
    };
  in {
    ### NixOS ###
    nixosModules.default = { lib, config, pkgs, ...}:
    let
      cfg = config.services.nix-sweep;
    in {
      options.services.nix-sweep = mkOptions {
        inherit lib pkgs;
        defaultProfiles = [ "system" ];
      };

      config = lib.mkIf cfg.enable {
        systemd.timers = {
          "nix-sweep" = {
            wantedBy = [ "timers.target" ];
            timerConfig = {
              OnCalendar = cfg.interval;
              Unit = "nix-sweep.service";
            };
          };

          "nix-sweep-gc" = lib.mkIf (cfg.gc && cfg.gcInterval != cfg.interval) {
            wantedBy = [ "timers.target" ];
            timerConfig = {
              OnCalendar = cfg.gcInterval;
              Unit = "nix-sweep-gc.service";
            };
          };
        };

        systemd.services = let
          scripts = mkServiceScripts { inherit lib cfg; };
        in {
          "nix-sweep" = {
            script = scripts.nix-sweep;
            serviceConfig = {
              Type = "oneshot";
              User = "root";
            };
          };

          "nix-sweep-gc" = lib.mkIf (cfg.gc && cfg.gcInterval != cfg.interval) {
            script = scripts.nix-sweep-gc;
            serviceConfig = {
              Type = "oneshot";
              User = "root";
            };
          };
        };
      };
    };

    ### Home Manager ###
    homeModules.default = { lib, config, pkgs, ...}:
    let
      cfg = config.services.nix-sweep;
    in {
      options.services.nix-sweep = mkOptions {
        inherit lib pkgs;
        defaultProfiles = [ "home" "user" ];
      };

      config = lib.mkIf cfg.enable {
        systemd.user.timers = {
          "nix-sweep" = {
            Install.WantedBy = [ "timers.target" ];
            Timer = {
              OnCalendar = cfg.interval;
              Unit = "nix-sweep.service";
            };
          };

          "nix-sweep-gc" = lib.mkIf (cfg.gc && cfg.gcInterval != cfg.interval) {
            Install.WantedBy = [ "timers.target" ];
            Timer = {
              OnCalendar = cfg.gcInterval;
              Unit = "nix-sweep-gc.service";
            };
          };
        };

        systemd.user.services = let
          scripts = mkServiceScripts { inherit lib cfg; };
        in {
          "nix-sweep".Service = {
            ExecStart = scripts.nix-sweep;
            Type = "oneshot";
          };

          "nix-sweep-gc".Service = {
            ExecStart = scripts.nix-sweep-gc;
            Type = "oneshot";
          };
        };
      };
    };
  });
}
