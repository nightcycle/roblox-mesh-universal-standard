"""Build deterministic Rokit release ZIPs and validate matching package versions."""
import hashlib
import pathlib
import sys
import tomllib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
TARGETS = {
    "windows-x86_64": "x86_64-pc-windows-msvc",
    "linux-x86_64": "x86_64-unknown-linux-gnu",
    "macos-x86_64": "x86_64-apple-darwin",
    "macos-aarch64": "aarch64-apple-darwin",
}

def validate(tag, root=ROOT):
    root = pathlib.Path(root)
    rust = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    wally = tomllib.loads((root / "mesh-runtime/wally.toml").read_text())["package"]["version"]
    if tag != f"v{rust}" or rust != wally:
        raise SystemExit(f"Tag {tag}, Rust {rust} and Wally {wally} must match")
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    versions = [
        package["version"] for package in lock["package"]
        if package["name"] == "roblox-mesh-universal-standard"
    ]
    if versions != [rust]:
        raise SystemExit("Cargo.lock must contain the matching package version; run cargo check before tagging")
    return rust

def package(tag, platform):
    validate(tag)
    target = TARGETS[platform]
    exe = "rmus.exe" if platform.startswith("windows") else "rmus"
    binary = ROOT / "target" / target / "release" / exe
    out = ROOT / "dist"
    out.mkdir(exist_ok=True)
    archive = out / f"rmus-{tag}-{platform}.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for path, name in [(binary, exe), (ROOT / "LICENSE", "LICENSE"), (ROOT / "README.md", "README.md")]:
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o100755 if name == exe else 0o100644) << 16
            z.writestr(info, path.read_bytes())
    print(archive)

def checksums(directory):
    directory = pathlib.Path(directory)
    files = sorted(
        p for p in directory.iterdir()
        if p.is_file() and p.name != "SHA256SUMS" and p.suffix != ".lock"
    )
    (directory / "SHA256SUMS").write_text(
        "".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in files)
    )

if __name__ == "__main__":
    command, *args = sys.argv[1:]
    {"validate": validate, "package": package, "checksums": checksums}[command](*args)
