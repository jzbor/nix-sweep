{
  description = "Cleanup old nix generations";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    cf.url = "github:jzbor/cornflakes";
  };

  outputs = { self, nixpkgs, cf, crane, ... }: (cf.mkLib nixpkgs).flakeForDefaultSystems (system:
  let
    pkgs = nixpkgs.legacyPackages.${system};
    craneLib = crane.mkLib pkgs;
  in {
    packages.default = craneLib.buildPackage {
      src = craneLib.cleanCargoSource ./.;
      strictDeps = true;
    };

    devShells.default = craneLib.devShell {
      inherit (self.packages.${system}.default) name;

      # Additional tools
      nativeBuildInputs = [];
    };

    nixosModules.default = { lib, config, pkgs, ...}: let
      cfg = config.services.nix-sweep;
    in {
      options.services.nix-sweep = {
        enable = lib.mkEnableOption "Enable nix-sweep";

        package = lib.mkOption {
          type = lib.types.package;
          default = self.packages.${pkgs.system}.default;
          description = "nix-sweep package to use for the service";
        };

        interval = lib.mkOption {
          type = lib.types.str;
          default = "daily";
          description = "How often to run nix-sweep (see systemd.time(7) for the format).";
        };
      };

      config = lib.mkIf cfg.enable {
        systemd.timers."nix-sweep" = {
          wantedBy = [ "timers.target" ];
          timerConfig = {
            OnCalendar = cfg.interval;
            Unit = "nix-sweep.service";
          };
        };

        systemd.services.nix-sweep = {
          script = "${cfg.package}/bin/nix-sweep --rm --system";
          serviceConfig = {
            Type = "oneshot";
            User = "root";
          };
        };
      };
    };
  });
}
