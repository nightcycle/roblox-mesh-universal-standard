"""Prepare Wally's auth.toml from a token or full config, without logging secrets."""
import json
import os
from pathlib import Path
import re
import sys
import tomllib

PUBLIC_API = "https://api.wally.run/"


def auth_config(secret):
    value = secret.strip()
    if not value:
        raise ValueError("WALLY_AUTH is empty; configure the repository secret before publishing")

    if re.fullmatch(r"[A-Za-z0-9_.-]+", value):
        tokens = {PUBLIC_API: value}
    else:
        try:
            tokens = tomllib.loads(value).get("tokens")
        except tomllib.TOMLDecodeError:
            # Parser diagnostics may include input. Never echo secret contents.
            raise ValueError("WALLY_AUTH must be a raw token or a valid auth.toml with a [tokens] table") from None

    if not isinstance(tokens, dict) or not all(
        isinstance(key, str) and isinstance(token, str) and token.strip()
        for key, token in tokens.items()
    ):
        raise ValueError("WALLY_AUTH must contain a [tokens] table with nonempty string values")
    if PUBLIC_API not in tokens:
        raise ValueError("WALLY_AUTH has no token for the public Wally registry")

    return "[tokens]\n" + "".join(
        f"{json.dumps(key)} = {json.dumps(token)}\n"
        for key, token in sorted(tokens.items())
    )


def write_auth(secret, path):
    contents = auth_config(secret)
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
        stream.write(contents)
    path.chmod(0o600)


if __name__ == "__main__":
    try:
        write_auth(os.environ.get("WALLY_AUTH", ""), sys.argv[1])
    except (ValueError, OSError) as error:
        print(f"Wally authentication setup failed: {error}", file=sys.stderr)
        sys.exit(1)
