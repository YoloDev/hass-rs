{
  nixConfig = {
    extra-substituters = [ "https://om.cachix.org" ];
    extra-trusted-public-keys = [ "om.cachix.org-1:ifal/RLZJKN4sbpScyPGqJ2+appCslzu7ZZF/C01f2Q=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    omnix = {
      url = "github:juspay/omnix";
      # We do not follow nixpkgs here, because then we can't use the omnix cache
      # inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      flake-utils,
      pre-commit-hooks,
      crane,
      rust-overlay,
      omnix,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [
            (import rust-overlay)
            (final: prev: {
              inherit (omnix.packages.${final.system}) omnix-cli;
            })
          ];
        };
        lib = pkgs.lib;
        toolchain = pkgs.rust-bin.stable.latest.default;
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        preCommitHooksLib = pre-commit-hooks.lib.${system};

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        src = craneLib.cleanCargoSource ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            cmake
            pkg-config
          ];

          buildInputs =
            with pkgs;
            [
              # Add additional build inputs here
              openssl
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
            ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        pre-commit-check = preCommitHooksLib.run {
          src = ./.;
          hooks = {
            flake-checker.enable = true;

            clippy = {
              enable = true;
              package = toolchain;
              settings.denyWarnings = true;
              settings.extraArgs = "--all";
              settings.offline = false;
            };

            nixfmt-rfc-style.enable = true;
          };
        };

      in
      rec {
        checks = {
          # Run clippy (and deny all warnings) on the workspace source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          workspace-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          # Check formatting
          workspace-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          inherit checks;
          inherit (pre-commit-check) shellHook;

          buildInputs = pre-commit-check.enabledPackages;

          packages = with pkgs; [
            cargo-autoinherit
            cargo-expand
            cargo-workspaces
            cargo-nextest
            just
            jq
            omnix-cli
          ];
        };
      }
    )

    # CI configuration
    // {
      om.ci = {
        default = {
          root = {
            dir = ".";
            steps = {
              # The build step is enabled by default. It builds all flake outputs.
              build.enable = true;
            };
          };
        };
      };
    };
}
