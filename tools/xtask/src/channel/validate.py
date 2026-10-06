"""Install generated system packages against this build's loopback archives."""

import functools
import http.server
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import threading


def run(*arguments):
    subprocess.run(arguments, check=True)


channels = pathlib.Path(sys.argv[1]).resolve()
assets = pathlib.Path(sys.argv[2]).resolve()
server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", 0),
    functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(assets)),
)
threading.Thread(target=server.serve_forever, daemon=True).start()
try:
    with tempfile.TemporaryDirectory() as temporary:
        local = pathlib.Path(temporary)
        shutil.copytree(channels, local / "channels")
        for relative in ("homebrew/sprawling.rb", "aur/PKGBUILD", "aur/.SRCINFO"):
            definition = local / "channels" / relative
            if definition.is_file():
                definition.write_text(
                    re.sub(
                        r"https://[^\s\"']+/releases/download/[^/]+/",
                        f"http://127.0.0.1:{server.server_port}/",
                        definition.read_text(encoding="utf-8"),
                    ),
                    encoding="utf-8",
                )
        run("brew", "tap-new", "--no-git", "local/channels")
        tap = subprocess.check_output(
            ["brew", "--repository", "local/channels"], text=True
        ).strip()
        shutil.copyfile(
            local / "channels/homebrew/sprawling.rb",
            pathlib.Path(tap) / "Formula/sprawling.rb",
        )
        run("brew", "install", "--formula", "local/channels/sprawling")
        prefix = subprocess.check_output(["brew", "--prefix", "sprawling"], text=True).strip()
        run(str(pathlib.Path(prefix) / "bin/sprawling"), "--version")
        run("brew", "test", "sprawling")
        run("brew", "uninstall", "sprawling")
        if (local / "channels/aur").is_dir():
            run(
                "docker", "run", "--rm", "--network=host",
                "-v", f"{local / 'channels/aur'}:/input:ro",
                "archlinux:base-devel", "bash", "-euc",
                """
                pacman -Syu --noconfirm --needed unzip git
                cp -a /input /work
                useradd -m builder
                chown -R builder:builder /work
                cd /work
                su builder -c 'cd /work && makepkg --printsrcinfo' > actual.SRCINFO
                diff -u .SRCINFO actual.SRCINFO
                su builder -c 'cd /work && makepkg --noconfirm'
                pacman -U --noconfirm ./*.pkg.tar.zst
                sprawling --version
                """,
            )
finally:
    server.shutdown()
    server.server_close()
