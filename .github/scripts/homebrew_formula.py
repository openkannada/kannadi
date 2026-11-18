#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 The Open Kannada Authors
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Writes the Homebrew formula for a kannadi release to standard output.

Linux gets the static musl builds, which run on any glibc.

    homebrew_formula.py VERSION SHA256_SUM [BASE_URL]

SHA256_SUM is the release's sha256.sum ("HASH *FILE" per line). BASE_URL defaults
to the release's download folder on GitHub.
"""

import sys

TARGETS = {
    ("mac", "arm"): "aarch64-apple-darwin",
    ("mac", "intel"): "x86_64-apple-darwin",
    ("linux", "arm"): "aarch64-unknown-linux-musl",
    ("linux", "intel"): "x86_64-unknown-linux-musl",
}

TEMPLATE = """\
class Kannadi < Formula
  desc "Convert Kannada documents in legacy fonts such as Nudi to Unicode, and back"
  homepage "https://github.com/openkannada/kannadi"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
{mac_arm}
    end
    on_intel do
{mac_intel}
    end
  end

  on_linux do
    on_arm do
{linux_arm}
    end
    on_intel do
{linux_intel}
    end
  end

  def install
    bin.install "kannadi"
    man1.install Dir["man/*.1"]
    bash_completion.install "completions/kannadi.bash" => "kannadi"
    zsh_completion.install "completions/_kannadi"
    fish_completion.install "completions/kannadi.fish"
  end

  test do
    assert_match "kannadi #{{version}}", shell_output("#{{bin}}/kannadi --version")
    (testpath/"a.txt").binwrite "P\\xC0\\xA3\\xC0\\xDF\\x71\\xC0".b
    system bin/"kannadi", "to-unicode", "a.txt", "--out", "out", "--strict"
    assert_equal "ಕನ್ನಡ", (testpath/"out/a.txt").read
  end
end
"""


def read_sums(path):
    sums = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            parts = line.split()
            if len(parts) == 2:
                sums[parts[1].lstrip("*")] = parts[0]
    return sums


def render(sums, base_url):
    blocks = {}
    for (os_name, cpu), target in TARGETS.items():
        archive = f"kannadi-cli-{target}.tar.xz"
        if archive not in sums:
            raise SystemExit(f"error: {archive} is not in the checksums")
        blocks[f"{os_name}_{cpu}"] = (
            f'      url "{base_url}/{archive}"\n      sha256 "{sums[archive]}"'
        )
    return TEMPLATE.format(**blocks)


def main(argv):
    if len(argv) not in (3, 4):
        raise SystemExit(__doc__)
    version, sums_path = argv[1], argv[2]
    base_url = argv[3] if len(argv) == 4 else (
        f"https://github.com/openkannada/kannadi/releases/download/v{version}"
    )
    sys.stdout.write(render(read_sums(sums_path), base_url))


if __name__ == "__main__":
    main(sys.argv)
