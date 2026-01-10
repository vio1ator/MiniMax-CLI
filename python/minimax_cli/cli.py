import os
import platform
import sys
import tarfile
import zipfile
from pathlib import Path
from urllib.request import urlopen

from minimax_cli import __version__

REPO = "Hmbown/MiniMax-CLI"


def main() -> None:
    binary = resolve_binary()
    os.execv(binary, [binary, *sys.argv[1:]])


def resolve_binary() -> str:
    override = os.getenv("MINIMAX_CLI_PATH")
    if override and Path(override).exists():
        return override

    cache_dir = Path.home() / ".minimax" / "bin" / __version__
    target, archive_ext = detect_target()
    bin_name = "minimax-cli.exe" if os.name == "nt" else "minimax-cli"
    dest = cache_dir / target / bin_name

    if dest.exists():
        return str(dest)

    if os.getenv("MINIMAX_CLI_SKIP_DOWNLOAD") in ("1", "true", "TRUE"):
        raise RuntimeError("minimax-cli binary not found and downloads are disabled.")

    url = (
        f"https://github.com/{REPO}/releases/download/v{__version__}/"
        f"minimax-cli-{__version__}-{target}.{archive_ext}"
    )
    download_and_extract(url, dest, archive_ext)
    return str(dest)


def detect_target() -> tuple[str, str]:
    system = platform.system().lower()
    arch = platform.machine().lower()

    if system == "linux" and arch in ("x86_64", "amd64"):
        return "x86_64-unknown-linux-gnu", "tar.gz"
    if system == "darwin" and arch in ("arm64", "aarch64"):
        return "aarch64-apple-darwin", "tar.gz"
    if system == "darwin" and arch in ("x86_64", "amd64"):
        return "x86_64-apple-darwin", "tar.gz"
    if system == "windows" and arch in ("x86_64", "amd64"):
        return "x86_64-pc-windows-msvc", "zip"

    raise RuntimeError(f"Unsupported platform: {system}/{arch}")


def download_and_extract(url: str, dest: Path, archive_ext: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = dest.with_suffix(".download")

    with urlopen(url) as response, open(tmp_path, "wb") as handle:
        handle.write(response.read())

    if archive_ext == "zip":
        with zipfile.ZipFile(tmp_path) as archive:
            archive.extractall(dest.parent)
    else:
        with tarfile.open(tmp_path, "r:gz") as archive:
            archive.extractall(dest.parent)

    tmp_path.unlink(missing_ok=True)

    if not dest.exists():
        raise RuntimeError("Binary not found in release archive.")

    if os.name != "nt":
        dest.chmod(dest.stat().st_mode | 0o111)
