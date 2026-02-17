{
  description = "Convert git repositories into beautifully formatted, printer-friendly PDFs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        cargoHash = "sha256-5nSy/RY9bQtGS/jbgtNhreC7lHLa0E/MqOW8cCEgv9c=";

        gitprint = pkgs.rustPlatform.buildRustPackage {
          pname = "gitprint";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;

          inherit cargoHash;

          nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];

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
        };
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

          clippy = pkgs.rustPlatform.buildRustPackage {
            pname = "gitprint-clippy";
            version = "0.1.0";
            src = pkgs.lib.cleanSource ./.;
  
            inherit cargoHash;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.clippy ];
            buildPhase = "cargo clippy --all-targets -- -D warnings";
            installPhase = "touch $out";
          };

          tests = pkgs.rustPlatform.buildRustPackage {
            pname = "gitprint-tests";
            version = "0.1.0";
            src = pkgs.lib.cleanSource ./.;
  
            inherit cargoHash;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.git ];
            buildPhase = ''
              export HOME=$(mktemp -d)
              cargo test --all
            '';
            installPhase = "touch $out";
          };

          fmt = pkgs.runCommand "gitprint-fmt-check" {
            nativeBuildInputs = [ rustToolchain ];
            src = pkgs.lib.cleanSource ./.;
          } ''
            cd $src
            cargo fmt -- --check
            touch $out
          '';
        };
      }
    ) // {
      overlays.default = final: prev: {
        gitprint = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    };
}
