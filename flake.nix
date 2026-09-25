# A devshell and `nix run`, and deliberately nothing else.
#
# NixOS has no `/lib64/ld-linux-x86-64.so.2`, so it cannot start a
# dynamically linked artifact and cannot use rustup's downloaded
# toolchains either. This flake exists so that a person on NixOS can enter
# a shell where `just check` works, and can run the binary once without
# installing anything. It does NOT take over the Windows or macOS release
# paths: those are `release.yml`, they are what a person downloads, and a
# second way to produce them would be a second authority for one artifact.
#
# THE LOAD-BEARING RULE: the Rust version lives in `rust-toolchain.toml`
# and nowhere else. `fromRustupToolchainFile` reads that file, so nothing
# here restates a version; `checks.toolchain-version-is-derived` then makes
# a flake that wants a version the file does not name turn the build red,
# rather than letting it drift quietly.
#
# `nix run` builds without the WebAssembly client bundle, so the binary it
# produces serves the page shell alone (`crates/sprawling/build.rs` says so
# at build time and `CLIENT_COMPLETE` is false). That is a development run.
# The archive `xtask package` assembles is refused unless it carries the
# real client, which is the check that keeps an empty browser window out of
# a release.
{
  description = "sprawling - a locally deployed agent-city harness";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # oxalica's overlay rather than fenix, for one reason: its
    # `fromRustupToolchainFile` needs no companion `sha256`, so the flake
    # names the file and nothing else. fenix's `fromToolchainFile` wants a
    # hash beside the file, which is a second thing to keep in step.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        toolchainFile = ./rust-toolchain.toml;

        # The one authority. Channel, profile and components all come from
        # the file the rest of the repository already obeys; `rustup show`
        # in CI and this expression read the same eleven lines.
        toolchain = pkgs.rust-bin.fromRustupToolchainFile toolchainFile;

        # Read back only so the check below can compare what nix built
        # against what the file asked for. This is a test reading a fact,
        # not a second place the version is decided.
        declaredChannel = (builtins.fromTOML (builtins.readFile toolchainFile)).toolchain.channel;

        # `just check` needs these on PATH; a devshell that stops short of
        # the repository's own closing condition is not a devshell. Which
        # tools those are is decided by the `prereqs` recipe in the
        # `justfile` and read back by `checks.devshell-covers-just-check`
        # below, so this list answers a list rather than being a second
        # one.
        tools = [
          pkgs.just
          pkgs.cargo-nextest
          pkgs.cargo-public-api
          pkgs.bun
          pkgs.cargo-deny
          pkgs.git
        ];

        # The one required row this shell cannot answer, named here so
        # that it stays one. `nightly-rustdoc` is what cargo-public-api
        # reads, it arrives through rustup, and NixOS cannot start a
        # rustup-downloaded toolchain at all - which is the reason this
        # flake exists, stated at the head of the file. So the `apisync`
        # gate is the single part of `just check` a NixOS shell does not
        # hold, and CI's `gates` job is where it is held.
        uncoveredRows = [ "nightly-rustdoc" ];

        # aws-lc-sys compiles the AWS-LC C sources, so a shell without cmake
        # and a C compiler fails on `cargo build` rather than on anything a
        # person changed. `reqwest` reaches it through rustls.
        nativeDeps = [
          pkgs.cmake
          pkgs.pkg-config
        ];

        sprawling = pkgs.rustPlatform.buildRustPackage {
          pname = "sprawling";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          src = self;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = nativeDeps ++ [ toolchain ];
          cargoBuildFlags = [ "-p" "sprawling" ];
          # The suite belongs to `just check` and to CI, which run it with
          # `--all-features` against a warm cache. Repeating it inside a
          # `nix build` buys no new verdict and costs a cold compile.
          doCheck = false;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [ toolchain ] ++ tools ++ nativeDeps;
          shellHook = ''
            echo "sprawling devshell - rustc from rust-toolchain.toml (${declaredChannel})"
            echo "the closing condition is: just check"
          '';
        };

        packages.default = sprawling;

        apps.default = {
          type = "app";
          program = "${sprawling}/bin/sprawling";
        };

        # A flake wanting a Rust version `rust-toolchain.toml` does not
        # name makes this red. It asks the built toolchain what it is and
        # compares that with the channel the file declares, so a version
        # pasted into this file - or an overlay silently resolving to
        # something else - fails here instead of surfacing as a warning
        # somebody has to notice.
        checks.toolchain-version-is-derived =
          pkgs.runCommand "toolchain-version-is-derived"
            {
              nativeBuildInputs = [ toolchain ];
              declared = declaredChannel;
            }
            ''
              set -eu
              built=$(rustc --version)
              echo "rust-toolchain.toml declares: $declared"
              echo "the flake's toolchain reports: $built"
              case "$built" in
                *"$declared"*) ;;
                *)
                  echo "the flake's Rust is not the one rust-toolchain.toml names" >&2
                  echo "derive it from that file; never restate a version here" >&2
                  exit 1
                  ;;
              esac
              touch "$out"
            '';

        # A devshell that stops short of `just check` fails here rather
        # than at the desk of the person who entered it. `just prereqs
        # list` prints one `class<TAB>name` row per tool, this check
        # looks each required row up on the PATH the devshell builds, and
        # a row added to the justfile without a package beside it in
        # `tools` turns red. The exception list is checked in the other
        # direction too: an entry that is no longer a required row is a
        # stale excuse and fails, so it cannot outlive the tool.
        checks.devshell-covers-just-check =
          pkgs.runCommand "devshell-covers-just-check"
            {
              nativeBuildInputs = [ toolchain pkgs.just ] ++ tools ++ nativeDeps;
              uncovered = pkgs.lib.concatStringsSep " " uncoveredRows;
            }
            ''
              set -eu
              tab=$(printf '\t')
              just --justfile ${self}/justfile --working-directory . prereqs list > rows
              missing=""
              while IFS="$tab" read -r class name; do
                [ "$class" = required ] || continue
                case " $uncovered " in
                  *" $name "*) echo "not covered by nix, by decision: $name"; continue ;;
                esac
                command -v "$name" >/dev/null 2>&1 || missing="$missing $name"
              done < rows
              for name in $uncovered; do
                grep -qxF "required$tab$name" rows || {
                  echo "$name is no longer a required row; drop it from uncoveredRows" >&2
                  exit 1
                }
              done
              if [ -n "$missing" ]; then
                echo "the devshell does not answer:$missing" >&2
                echo "add a package for each to 'tools' in flake.nix, or record why nix cannot" >&2
                exit 1
              fi
              echo "every required row of 'just prereqs' answers in this shell"
              touch "$out"
            '';

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
