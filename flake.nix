{
  description = "Convert git repositories into beautifully formatted, printer-friendly PDFs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Pin to the exact version declared in rust-toolchain.toml — same as CI.
        # Dev shell adds rust-src + rust-analyzer on top for IDE support.
        rustToolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Source filtering: include Rust/Cargo files + embedded fonts
        unfilteredRoot = ./.;
        src = pkgs.lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = pkgs.lib.fileset.unions [
            (craneLib.fileset.commonCargoSources unfilteredRoot)
            (pkgs.lib.fileset.fileFilter (file: file.hasExt "ttf") unfilteredRoot)
          ];
        };

        commonArgs = {
          inherit src;
          nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
        };

        # Build dependencies only — cached separately from source changes
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        gitprint = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          nativeCheckInputs = [ pkgs.git ];

          preCheck = ''
            export HOME=$(mktemp -d)
          '';

          postInstall = ''
            wrapProgram $out/bin/gitprint \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.git ]}
          '';

          meta = with pkgs.lib; {
            description = "Convert git repositories into beautifully formatted, printer-friendly PDFs";
            homepage = "https://github.com/izelnakri/gitprint";
            license = licenses.mit;
            mainProgram = "gitprint";
          };
        });
      in
      {
        packages = {
          default = gitprint;
          gitprint = gitprint;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = gitprint;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.git
            pkgs.cargo-watch
            pkgs.cargo-edit
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };

        checks = {
          inherit gitprint;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });

          fmt = craneLib.cargoFmt {
            inherit src;
          };
        };
      }
    ) // {
      overlays.default = final: prev: {
        gitprint = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    };
}
