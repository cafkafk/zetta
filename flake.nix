{
  description = "eza: a modern, maintained replacement for ls";

  inputs = {
    nixpkgs.url = "http:/rime.cx/v1/github/NixOS/nixpkgs/b/nixpkgs-unstable.tar.gz";

    flake-utils = {
      url = "http://rime.cx/v1/github/semnix/flake-utils.tar.gz";
    };

    naersk = {
      url = "http://rime.cx/v1/github/semnix/naersk.tar.gz";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "http://rime.cx/v1/github/semnix/rust-overlay.tar.gz";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "http://rime.cx/v1/github/semnix/treefmt-nix.tar.gz";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    powertest = {
      url = "http://rime.cx/v1/github/eza-community/powertest.tar.gz";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        naersk.follows = "naersk";
        treefmt-nix.follows = "treefmt-nix";
        rust-overlay.follows = "rust-overlay";
      };
    };

    pre-commit-hooks = {
      url = "http://rime.cx/v1/github/semnix/pre-commit-hooks.nix.tar.gz";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = {
    self,
    flake-utils,
    naersk,
    nixpkgs,
    treefmt-nix,
    rust-overlay,
    powertest,
    pre-commit-hooks,
    ...
  }: let
    inherit (builtins) fromTOML readFile substring;

    cargoToml = fromTOML (readFile ./Cargo.toml);
    version = "${cargoToml.package.version}+${substring 0 8 self.lastModifiedDate}_${self.shortRev or "dirty"}";

    mkToolchain = pkgs: pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

    mkNaersk = pkgs: let
      toolchain = mkToolchain pkgs;
    in
      pkgs.callPackage naersk {
        cargo = toolchain;
        rustc = toolchain;
        clippy = toolchain;
      };

    mkEzaBuildInputs = pkgs:
      with pkgs;
        [zlib]
        ++ lib.optionals stdenv.isDarwin [libiconv darwin.apple_sdk.frameworks.Security];

    mkEza = pkgs: let
      naersk' = mkNaersk pkgs;
      buildInputs = mkEzaBuildInputs pkgs;
    in
      naersk'.buildPackage rec {
        pname = "eza";
        inherit version;

        src = ./.;
        doCheck = true; # run `cargo test` on build

        inherit buildInputs;
        nativeBuildInputs = with pkgs; [cmake pkg-config installShellFiles pandoc];

        buildNoDefaultFeatures = true;
        buildFeatures = "git";

        postInstall = ''
          pandoc --standalone -f markdown -t man <(cat "man/eza.1.md" | sed "s/\$version/${version}/g") > man/eza.1
          pandoc --standalone -f markdown -t man <(cat "man/eza_colors.5.md" | sed "s/\$version/${version}/g") > man/eza_colors.5
          pandoc --standalone -f markdown -t man <(cat "man/eza_colors-explanation.5.md" | sed "s/\$version/${version}/g")> man/eza_colors-explanation.5
          installManPage man/eza.1 man/eza_colors.5 man/eza_colors-explanation.5
          installShellCompletion \
            --bash completions/bash/eza \
            --fish completions/fish/eza.fish \
            --zsh completions/zsh/_eza
        '';

        meta = with pkgs.lib; {
          description = "A modern, maintained replacement for ls";
          longDescription = ''
            eza is a modern replacement for ls. It uses colours for information by
            default, helping you distinguish between many types of files, such as
            whether you are the owner, or in the owning group. It also has extra
            features not present in the original ls, such as viewing the Git status
            for a directory, or recursing into directories with a tree view. eza is
            written in Rust, so it’s small, fast, and portable.
          '';
          homepage = "https://github.com/eza-community/eza";
          license = licenses.mit;
          mainProgram = "eza";
          maintainers = with maintainers; [cafkafk];
        };
      };

    mkEzaDevPackages = pkgs: let
      buildInputs = mkEzaBuildInputs pkgs;
      naersk' = mkNaersk pkgs;
    in {
      # Check code
      check = naersk'.buildPackage {
        src = ./.;
        mode = "check";
        inherit buildInputs;
      };

      # Run tests
      test = naersk'.buildPackage {
        src = ./.;
        mode = "test";
        inherit buildInputs;
      };

      # Lint code
      clippy = naersk'.buildPackage {
        src = ./.;
        mode = "clippy";
        inherit buildInputs;
      };

      # Run integration tests
      trycmd = naersk'.buildPackage {
        src = ./.;
        mode = "test";
        doCheck = true;
        # No reason to wait for release build
        release = false;
        # buildPhase files differ between dep and main phase
        singleStep = true;
        # generate testing files
        buildPhase = ''bash devtools/dir-generator.sh tests/test_dir && echo "Dir generated"'';
        cargoTestOptions = opts: opts ++ ["--features nix"];
        inherit buildInputs;
        nativeBuildInputs = with pkgs; [git];
      };

      # TODO: add conditionally to checks.
      # Run integration tests
      trycmd-local = naersk'.buildPackage {
        src = ./.;
        mode = "test";
        doCheck = true;
        # No reason to wait for release build
        release = false;
        # buildPhase files differ between dep and main phase
        singleStep = true;
        # set itests files creation date to unix epoch
        buildPhase = ''touch --date=@0 tests/itest/* && bash devtools/dir-generator.sh tests/test_dir'';
        cargoTestOptions = opts: opts ++ ["--features nix" "--features nix-local" "--features powertest"];
        inherit buildInputs;
        nativeBuildInputs = with pkgs; [git];
      };

      # Dump testing files
      trydump = naersk'.buildPackage {
        src = ./.;
        mode = "test";
        doCheck = true;
        # No reason to wait for release build
        release = false;
        # buildPhase files differ between dep and main phase
        singleStep = true;
        # set itests files creation date to unix epoch
        buildPhase = ''
          bash devtools/dir-generator.sh tests/test_dir
          touch --date=@0 tests/itest/*;
          rm tests/cmd/*.stdout || echo;
          rm tests/cmd/*.stderr || echo;

          touch --date=@0 tests/ptests/*;
          rm tests/ptests/*.stdout || echo;
          rm tests/ptests/*.stderr || echo;
        '';
        cargoTestOptions = opts: opts ++ ["--features nix" "--features nix-local" "--features powertest"];
        TRYCMD = "dump";
        postInstall = ''
          cp dump $out -r
        '';
        inherit buildInputs;
        nativeBuildInputs = with pkgs; [git];
      };
    };
  in
    {
      overlays.default = _final: prev: {
        eza = mkEza prev;
      };
    }
    // flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];

        pkgs = import nixpkgs {inherit system overlays;};

        treefmtEval = treefmt-nix.lib.evalModule pkgs ./treefmt.nix;
        eza = mkEza pkgs;
        devPkgs = mkEzaDevPackages pkgs;
      in {
        # For `nix fmt`
        formatter = treefmtEval.config.build.wrapper;

        packages = {
          # For `nix build` `nix run`, & `nix profile install`:
          default = eza;

          # Run `nix build .#check` to check code
          inherit (devPkgs) check;

          # Run `nix build .#test` to run tests
          inherit (devPkgs) test;

          # Run `nix build .#clippy` to lint code
          inherit (devPkgs) clippy;

          # Run `nix build .#trycmd` to run integration tests
          inherit (devPkgs) trycmd;

          # Run `nix build .#trycmd` to run integration tests
          inherit (devPkgs) trycmd-local;

          # Run `nix build .#trydump` to dump testing files
          inherit (devPkgs) trydump;
        };

        # For `nix develop`:
        devShells = {
          default = pkgs.mkShell {
            inherit (self.checks.${system}.pre-commit-check) shellHook;
            nativeBuildInputs = with pkgs; [
              rustup
              (mkToolchain pkgs)
              just
              pandoc
              convco
              zip

              # For generating demo
              vhs

              powertest.packages.${pkgs.system}.default

              cargo-hack
              cargo-udeps
              cargo-outdated
            ];
          };
          # `nix develop .#eza-dev` to build nix package locally.
          eza-dev = pkgs.mkShell {
            packages = [
              eza
            ];
          };
        };

        # For `nix flake check`
        checks = {
          pre-commit-check = let
            # some treefmt formatters are not supported in pre-commit-hooks we filter them out for now.
            toFilter =
              # This is a nice hack to not have to manually filter we should keep in mind for a future refactor.
              # (builtins.attrNames pre-commit-hooks.packages.${system})
              ["yamlfmt"];
            filterFn = n: _v: (!builtins.elem n toFilter);
            treefmtFormatters = pkgs.lib.mapAttrs (_n: v: {inherit (v) enable;}) (pkgs.lib.filterAttrs filterFn (import ./treefmt.nix).programs);
          in
            pre-commit-hooks.lib.${system}.run {
              src = ./.;
              hooks =
                treefmtFormatters
                // {
                  convco.enable = true; # not in treefmt
                };
            };
          formatting = treefmtEval.config.build.check self;
          build = devPkgs.check;
          default = eza;
          inherit (devPkgs) test trycmd;
          lint = devPkgs.clippy;
        };
      }
    );
}
