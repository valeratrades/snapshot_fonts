{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    v_flakes.url = "github:valeratrades/v_flakes/v1.6";
  };
  outputs = { self, nixpkgs, rust-overlay, flake-utils, pre-commit-hooks, v_flakes }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          allowUnfree = true;
        };
        #NB: can't load rust-bin from nightly.latest, as there are week guarantees of which components will be available on each day.
        rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rust-docs" "rustc-codegen-cranelift-preview" ];
        });
        pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        pname = manifest.name;
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

        rs = v_flakes.rs { inherit pkgs rust; };
        github = v_flakes.github {
          inherit pkgs pname rs;
          enable = true;
          lastSupportedVersion = "nightly-2025-12-12";
          jobs = { default = true; };
        };
        readme = v_flakes.readme-fw {
          inherit pkgs pname;
          lastSupportedVersion = "nightly-1.93";
          rootDir = ./.;
          defaults = true;
          badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ];
        };
        combined = v_flakes.utils.combine [ rs github readme ];
      in
      {
        packages =
          let
            rustc = rust;
            cargo = rust;
            rustPlatform = pkgs.makeRustPlatform {
              inherit rustc cargo stdenv;
            };
            snapshot_fonts_bin = rustPlatform.buildRustPackage rec {
              inherit pname;
              version = manifest.version;
              doCheck = false;

              buildInputs = with pkgs; [
                openssl.dev
              ];
              nativeBuildInputs = with pkgs; [ pkg-config ];

              cargoLock.lockFile = ./Cargo.lock;
              src = pkgs.lib.cleanSource ./.;
            };
          in
          {
            inherit snapshot_fonts_bin;

            default = pkgs.runCommand "snapshot-fonts"
              {
                nativeBuildInputs = [ snapshot_fonts_bin pkgs.fontforge ];
              } ''
              mkdir -p $out/share/fonts/truetype
              snapshot_fonts fill-levels --output $out/share/fonts/truetype/FillLevels.ttf
              snapshot_fonts candles --output $out/share/fonts/truetype/Candles.ttf
            '';

            fill-levels = pkgs.runCommand "fill-levels-font"
              {
                nativeBuildInputs = [ snapshot_fonts_bin pkgs.fontforge ];
              } ''
              mkdir -p $out/share/fonts/truetype
              snapshot_fonts fill-levels --output $out/share/fonts/truetype/FillLevels.ttf
            '';

            candles = pkgs.runCommand "candles-font"
              {
                nativeBuildInputs = [ snapshot_fonts_bin pkgs.fontforge ];
              } ''
              mkdir -p $out/share/fonts/truetype
              snapshot_fonts candles --output $out/share/fonts/truetype/Candles.ttf
            '';
          };

        devShells.default =
          with pkgs;
          mkShell {
            inherit stdenv;
            shellHook =
              pre-commit-check.shellHook
              + combined.shellHook
              + ''
                cp -f ${v_flakes.files.treefmt { inherit pkgs; }} ./.treefmt.toml
              '';

            packages = [
              mold
              openssl
              pkg-config
              rust
              fontforge
              uv
            ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;

            env.RUST_BACKTRACE = 1;
            env.RUST_LIB_BACKTRACE = 0;
          };
      }
    );
}
