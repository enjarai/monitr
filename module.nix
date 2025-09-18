{
  packages
}: (
  {
    lib,
    pkgs,
    config,
    ...
  }:

  let
    inherit (lib)
      mkEnableOption
      mkIf
      mkOption
      optionalAttrs
      optional
      mkPackageOption;
    inherit (lib.types)
      bool
      path
      str
      submodule
      number
      array
      listOf;

    cfg = config.services.monitr;
  in
  {
    options.services.monitr = {
      enable = mkEnableOption "Monitr";

      package = mkPackageOption packages.${pkgs.stdenv.hostPlatform.system} "default" { };

      user = mkOption {
        type = str;
        default = "monitr";
        description = "User account under which the bot runs.";
      };

      group = mkOption {
        type = str;
        default = "monitr";
        description = "Group account under which the bot runs.";
      };

      address = mkOption {
        type = str;
        default = "0.0.0.0";
        description = "The address to listen on.";
      };

      port = mkOption {
        type = number;
        default = 8098;
        description = "The port to listen on.";
      };

      token = mkOption {
        type = str;
        description = "The auth token.";
      };

      train_token = mkOption {
        type = str;
        description = "The token to use for the NS trains api.";
      };
    };

    config = mkIf cfg.enable {
      systemd.services = {
        monitr = {
          description = "Monitr";
          after = [ "network.target" ];
          wantedBy = [ "multi-user.target" ];
          restartTriggers = [
            cfg.package
            cfg.address
            cfg.port
          ];

          serviceConfig = {
            Type = "simple";
            User = cfg.user;
            Group = cfg.group;
            WorkingDirectory = cfg.package;
            ExecStart = "${cfg.package}/bin/monitr";
            Restart = "always";
          };

          environment = {
            ADDRESS = cfg.address;
            PORT = toString cfg.port;
            TOKEN = cfg.token;
            TRAIN_TOKEN = cfg.train_token;
          };
        };
      };

      users.users = optionalAttrs (cfg.user == "monitr") {
        monitr = {
          isSystemUser = true;
          group = cfg.group;
        };
      };

      users.groups = optionalAttrs (cfg.group == "monitr") {
        monitr = { };
      };
    };
  }
)