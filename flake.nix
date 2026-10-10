# The devshell and complete application share the repository toolchain.
# Packaging contract: sprawling D50 in crates/sprawling/spec/Install.lean.
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
    # nixpkgs has no Bun lock builder; this consumes the existing lock's
    # integrity hashes and keeps dependency fetching outside the sandbox.
    bun2nix = {
      url = "github:nix-community/bun2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, rust-overlay, flake-utils, bun2nix }:
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

        # Zig's version lives in `crates/desktop/ffi/zig-version` and
        # nowhere else, as Rust's lives in `rust-toolchain.toml`. nixpkgs
        # carries one attribute per Zig series (`zig_0_17` for 0.17.x), so
        # the attribute is derived from that file rather than named here.
        # The doctor's `zig` row probes for the exact pin, and
        # `checks.devshell-covers-just-check` below runs that probe, so a
        # nixpkgs whose series has moved to another patch release turns the
        # check red rather than handing the shell a Zig the build refuses.
        zigPin = pkgs.lib.strings.trim (builtins.readFile ./crates/desktop/ffi/zig-version);
        zigAttribute = "zig_" + builtins.replaceStrings [ "." ] [ "_" ] (pkgs.lib.versions.majorMinor zigPin);
        zig =
          pkgs.${zigAttribute}
            or (throw "nixpkgs has no ${zigAttribute}, the series crates/desktop/ffi/zig-version pins (${zigPin})");

        # `just check` needs these on PATH; a devshell that stops short of
        # the repository's own closing condition is not a devshell. Which
        # tools those are is decided by the `prereqs` recipe in the
        # `justfile` and read back by `checks.devshell-covers-just-check`
        # below, so this list answers a list rather than being a second
        # one.
        tools = [
          pkgs.just
          pkgs.cargo-nextest
          pkgs.bun
          pkgs.cargo-deny
          pkgs.git
          pkgs.elan
          pkgs.uv
          zig
        ];

        # The required rows this shell cannot answer, named here so that
        # they stay few. `rustup` is how every other platform gets the
        # toolchain `rust-toolchain.toml` pins, and NixOS cannot start a
        # rustup-downloaded toolchain at all - which is the reason this
        # flake exists, stated at the head of the file. The shell carries
        # that toolchain itself instead. `lean` is the toolchain
        # `lean-toolchain` pins: the shell carries elan, which fetches that
        # toolchain on first use, and the check below runs in a build
        # sandbox with no network, where no download can happen.
        uncoveredRows = [ "rustup" "lean" ];

        # aws-lc-sys compiles the AWS-LC C sources, so a shell without cmake
        # and a C compiler fails on `cargo build` rather than on anything a
        # person changed. `reqwest` reaches it through rustls.
        nativeDeps = [
          pkgs.cmake
          pkgs.pkg-config
        ];

        bunBuilder = bun2nix.packages.${system}.default;
        # Generated in the store on every lock change, never maintained as
        # another lock in the source tree. npm hashes come from bun.lock.
        bunNix = pkgs.runCommand "sprawling-bun-dependencies.nix"
          { nativeBuildInputs = [ bunBuilder pkgs.bun ]; }
          ''
            cat > project-lock.js <<'JS'
            const lock = Bun.JSONC.parse(await Bun.file(Bun.argv.at(-1)).text());
            if (lock.lockfileVersion !== 2 || lock.configVersion !== 1 ||
                !lock.packages || !Object.values(lock.packages).every(pkg =>
                  Array.isArray(pkg) && pkg.length === 4 &&
                  typeof pkg[0] === "string" && typeof pkg[1] === "string" &&
                  pkg[2] !== null && typeof pkg[2] === "object" &&
                  typeof pkg[3] === "string" && pkg[3].startsWith("sha512-"))) {
              console.error("Unsupported Bun lock shape; update the Nix conversion adapter");
              process.exit(1);
            }
            console.log(JSON.stringify({ lockfileVersion: 1, packages: lock.packages }));
            JS
            bun project-lock.js ${./client/bun.lock} > converter.lock
            bun2nix --lock-file converter.lock --output-file "$out"
          '';
        bunDeps = bunBuilder.fetchBunDeps { inherit bunNix; };
        # These readers fail if the authoritative declarations move, so a
        # packaging build cannot silently fall back to an obsolete path.
        bundleDir = builtins.elemAt
          (builtins.match ''.*const BUNDLE_DIR: &str = "([^"]+)";.*''
            (builtins.readFile ./crates/sprawling/build.rs)) 0;
        skillSource = builtins.elemAt
          (builtins.match ''.*const SKILLS_DIR: &str = "([^"]+)";.*''
            (builtins.readFile ./tools/xtask/src/package/contents.rs)) 0;
        skillEntry = builtins.elemAt
          (builtins.match ''.*const SKILLS_ENTRY: &str = "([^"]+)";.*''
            (builtins.readFile ./tools/xtask/src/package/contents.rs)) 0;
        applicationManifest = builtins.fromTOML (builtins.readFile ./crates/sprawling/Cargo.toml);
        workspacePackage = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package;
        applicationName = applicationManifest.package.name;
        mainProgram = (builtins.head (applicationManifest.bin or [ { name = applicationName; } ])).name;
        resourceDir = "share/${applicationName}";
        documentationDir = "share/doc/${applicationName}";
        rustPlatform = pkgs.makeRustPlatform { cargo = toolchain; rustc = toolchain; };

        sprawling = rustPlatform.buildRustPackage {
          pname = applicationName;
          version = workspacePackage.version;
          src = self;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          # zig builds the console's renderer, `crates/console_ffi`, on every
          # platform (`crates/console_ffi/Spec.lean` D1).
          nativeBuildInputs = nativeDeps ++ [ pkgs.bun pkgs.just zig ];
          # just is a command used by preBuild, not this package's builder.
          dontUseJustBuild = true;
          dontUseJustCheck = true;
          dontUseJustInstall = true;
          preBuild = ''
            export HOME="$TMPDIR/home"
            mkdir -p "$HOME"
            # Native addons are loaded into Bun; only the build needs GCC's
            # runtime libraries, not the installed Rust application.
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ]}"
            export BUN_INSTALL_CACHE_DIR="$TMPDIR/bun-cache"
            cp -R ${bunDeps}/share/bun-cache "$BUN_INSTALL_CACHE_DIR"
            chmod -R u+w "$BUN_INSTALL_CACHE_DIR"
            just build-web
            test -s crates/sprawling/${bundleDir}/index.html
          '';
          postBuild = ''
            # Judge what build.rs embedded, not just what Vite left on disk.
            find "''${CARGO_TARGET_DIR:-target}" -name client_embed.rs -print0               | xargs -0 grep -l '^pub const CLIENT_COMPLETE: bool = true;' > complete-client
            if ! test -s complete-client; then
              echo "Nix application has no embedded client; restore just build-web" >&2
              exit 1
            fi
          '';
          postInstall = ''
            mkdir -p "$out/${resourceDir}"
            cp -R ${skillSource} "$out/${resourceDir}/${skillEntry}"
            install -Dm644 LICENSE "$out/${documentationDir}/LICENSE"
          '';
          passthru = { inherit bundleDir skillEntry resourceDir documentationDir; };
          cargoBuildFlags = [ "-p" applicationName ];
          # The suite belongs to `just check` and to CI, which run it with
          # `--all-features` against a warm cache. Repeating it inside a
          # `nix build` buys no new verdict and costs a cold compile.
          doCheck = false;
          meta = {
            description = (builtins.fromTOML (builtins.readFile ./crates/sprawling/Cargo.toml)).package.description;
            homepage = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.homepage;
            license = pkgs.lib.meta.getLicenseFromSpdxIdOr workspacePackage.license
              (throw "Unsupported Cargo SPDX license: ${workspacePackage.license}");
            inherit mainProgram;
          };
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
          program = pkgs.lib.getExe sprawling;
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
        # list` prints one `class<TAB>name<TAB>probe` row per tool, this
        # check runs each required row's probe on the PATH the devshell
        # builds, skipping a `-` probe the doctor alone can answer, and a
        # row added to the doctor's table without a package beside it in
        # `tools` turns red. The exception list is checked in the other
        # direction too: an entry that is no longer a required row is a
        # stale excuse and fails, so it cannot outlive the tool.
        #
        # The build sandbox has no `/usr/bin/env`, which every shebang
        # recipe in the `justfile` names, so the check runs a copy of the
        # justfile whose shebangs point at the sandbox's own bash. The copy
        # keeps its place beside `prereqs.tsv`, because the recipe finds
        # that file relative to the justfile.
        checks.devshell-covers-just-check =
          pkgs.runCommand "devshell-covers-just-check"
            {
              nativeBuildInputs = [ toolchain pkgs.just ] ++ tools ++ nativeDeps;
              uncovered = pkgs.lib.concatStringsSep " " uncoveredRows;
            }
            ''
              set -eu
              tab=$(printf '\t')
              table=crates/sprawling/src/doctor/table
              mkdir -p tree/$table
              cp ${self}/justfile tree/justfile
              chmod u+w tree/justfile
              cp ${self}/$table/prereqs.tsv tree/$table/prereqs.tsv
              substituteInPlace tree/justfile \
                --replace-fail '#!/usr/bin/env bash' '#!${pkgs.runtimeShell}'
              just --justfile tree/justfile --working-directory . prereqs list > rows
              missing=""
              while IFS="$tab" read -r class name probe; do
                [ "$class" = required ] || continue
                [ "$probe" = - ] && continue
                case " $uncovered " in
                  *" $name "*) echo "not covered by nix, by decision: $name"; continue ;;
                esac
                eval "$probe" >/dev/null 2>&1 || missing="$missing $name"
              done < rows
              for name in $uncovered; do
                grep -q "^required$tab$name$tab" rows || {
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
