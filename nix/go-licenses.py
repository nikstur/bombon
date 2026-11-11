"""Add the licenses of the Go modules to the SBOMs of `cyclonedx-gomod bin`.

Usage: go-licenses.py <spdx-licenses.json> <modules> <sbom>...

`cyclonedx-gomod` finds licenses only after it downloads the modules, which
does not work offline. Thus this script finds them in <modules>, the modules
that the build vendors:

- A vendor directory (with `modules.txt`). A module that a `replace`
  directive changes to another module is in the directory of the original
  module.
- With `proxyVendor`, a module cache with a zip file for each module.

The licenses are in the license files in the root directory of each module.
Go modules declare no license, thus `identify_license` (`licenseclassifier`)
identifies the licenses from the text, also from a license notice
(`-headers`), such as the notice of `Apache-2.0`. A module with more than one
license gets them all with `AND`, the strict choice. In this expression, a
license that is not on the SPDX list is `LicenseRef-<name>`. A module without
a license file, or with a license that `identify_license` does not know, gets
no license.

The script changes each <sbom> in place.
"""

import json
import re
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

LICENSE_FILE = re.compile(r"(?i)^(licen[cs]e|copying)([.\-_].*)?$")
# The names of `identify_license` that are not the SPDX identifier of the
# license.
SPDX = {
    "BSD-0-Clause": "0BSD",
    "GNU-All-permissive-Copying-License": "FSFAP",
    "bzip2": "bzip2-1.0.6",
    "cURL": "curl",
    "tcl_tk": "TCL",
}


def escape(path):
    """The form of a module path in the module cache: `!` and the lower case
    letter for an upper case letter."""
    return re.sub(r"[A-Z]", lambda m: "!" + m.group().lower(), path)


def vendored(vendor):
    """The directory of each (path, version) in a vendor directory."""
    directories = {}
    for line in (vendor / "modules.txt").read_text().splitlines():
        if not line.startswith("# "):
            continue
        # `# <path> <version>`, or with a `replace` directive
        # `# <path> <version> => <path> <version>`.
        original, _, replacement = line[2:].partition(" => ")
        module = tuple((replacement or original).split())
        if len(module) == 2:
            directories[module] = vendor / original.split()[0]
    return directories


def license_files(modules, directories, path, version):
    """(name, bytes) of each license file in the root of a module."""
    if directories is not None:
        root = directories.get((path, version))
        if root and root.is_dir():
            for file in sorted(root.iterdir()):
                if LICENSE_FILE.match(file.name) and file.is_file():
                    yield file.name, file.read_bytes()
        return
    archive = modules / escape(path) / "@v" / f"{escape(version)}.zip"
    if not archive.is_file():
        return
    prefix = f"{path}@{version}/"
    with zipfile.ZipFile(archive) as zf:
        for name in sorted(zf.namelist()):
            rest = name[len(prefix) :] if name.startswith(prefix) else ""
            if rest and "/" not in rest and LICENSE_FILE.match(rest):
                yield rest, zf.read(name)


def identify(modules, found):
    """The set of licenses of each index in `found`."""
    directories = None
    if (modules / "modules.txt").is_file():
        directories = vendored(modules)
    licenses = {}
    with tempfile.TemporaryDirectory() as tmp:
        for index, (path, version) in enumerate(found):
            for name, text in license_files(modules, directories, path, version):
                directory = Path(tmp, str(index))
                directory.mkdir(exist_ok=True)
                (directory / name).write_bytes(text)
        if not any(Path(tmp).iterdir()):
            return licenses
        result = Path(tmp, "result.json")
        # It logs each file to stderr: keep that only on a failure.
        run = subprocess.run(
            ["identify_license", "-headers", "-json", result, tmp],
            capture_output=True,
        )
        if run.returncode:
            sys.stderr.buffer.write(run.stderr)
            sys.exit(run.returncode)
        for entry in json.loads(result.read_text()) or []:
            index = int(Path(entry["Filepath"]).relative_to(tmp).parts[0])
            for match in entry.get("Classifications") or []:
                if match["Name"] != "Copyright":
                    name = SPDX.get(match["Name"], match["Name"])
                    licenses.setdefault(index, set()).add(name)
    return licenses


def spdx_id(name, spdx_ids):
    """The identifier of a license in an SPDX expression."""
    if name in spdx_ids:
        return name
    # A `LicenseRef-` allows only letters, digits, `.` and `-`.
    return "LicenseRef-" + re.sub(r"[^A-Za-z0-9.-]", "-", name)


def cyclonedx(names, spdx_ids):
    """The CycloneDX `licenses` of a set of license names."""
    if len(names) > 1:
        ids = sorted(spdx_id(name, spdx_ids) for name in names)
        return [{"expression": " AND ".join(ids)}]
    (name,) = names
    return [{"license": {"id" if name in spdx_ids else "name": name}}]


def main(spdx_path, modules, *sbom_paths):
    spdx = json.loads(Path(spdx_path).read_text())
    spdx_ids = {entry["licenseId"] for entry in spdx["licenses"]}
    sboms = {path: json.loads(Path(path).read_text()) for path in sbom_paths}
    found = sorted(
        {
            (component["name"], component["version"])
            for sbom in sboms.values()
            for component in sbom.get("components") or []
            if "version" in component
        }
    )
    licenses = identify(Path(modules), found)
    by_module = {found[index]: names for index, names in licenses.items()}
    for path, sbom in sboms.items():
        for component in sbom.get("components") or []:
            names = by_module.get((component["name"], component.get("version")))
            if names:
                component["licenses"] = cyclonedx(names, spdx_ids)
        Path(path).write_text(json.dumps(sbom, indent=2) + "\n")
    print(f"found {len(found)} Go modules, {len(licenses)} with a license")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    main(*sys.argv[1:])
