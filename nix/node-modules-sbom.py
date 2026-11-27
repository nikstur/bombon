"""Write a CycloneDX SBOM of the npm packages in the `node_modules` directories of store paths.

Usage: node-modules-sbom.py NAME ROOT...

A package is a directory directly inside a `node_modules` directory (or inside a scope directory
like `node_modules/@scope`) that contains a `package.json` with a name and a version. The script
follows symlinks, so it also finds packages that are linked in from other store paths, and the
`.pnpm` layout of pnpm.

The dependency edges come from the `dependencies`, `optionalDependencies` and `peerDependencies`
of each `package.json`, resolved like Node.js does: the nearest `node_modules/<name>` directory in
the directory or in one of its parents. The SBOM root depends on the packages that no other
package depends on.
"""

import json
import os
import sys
from pathlib import Path
from urllib.parse import quote


def is_package_directory(directory: Path) -> bool:
    parent = directory.parent
    if parent.name.startswith("@"):
        parent = parent.parent
    return parent.name == "node_modules"


def license_name(value):
    if isinstance(value, dict):
        value = value.get("type")
    if isinstance(value, str) and value.strip():
        return value.strip()
    return None


def declared_license(manifest):
    """Old packages declare the license as an object or as a list in `licenses`.

    Like npm, interpret such a list as a choice between the licenses.
    """
    value = manifest.get("license") or manifest.get("licenses")
    if isinstance(value, list):
        names = [n for n in map(license_name, value) if n]
    else:
        names = [n for n in [license_name(value)] if n]
    if not names:
        return None
    if len(names) == 1:
        return names[0]
    return " OR ".join(f"({n})" if " " in n else n for n in names)


def read_manifest(directory: Path):
    try:
        manifest = json.loads((directory / "package.json").read_bytes())
    except (OSError, ValueError):
        return None
    if not isinstance(manifest, dict):
        return None
    name = manifest.get("name")
    version = manifest.get("version")
    if not (isinstance(name, str) and name and isinstance(version, str) and version):
        return None
    return manifest


def purl(name: str, version: str) -> str:
    if name.startswith("@") and "/" in name:
        scope, base = name.split("/", 1)
        path = f"{quote(scope, safe='')}/{quote(base, safe='')}"
    else:
        path = quote(name, safe="")
    return f"pkg:npm/{path}@{quote(version, safe='')}"


def component(manifest, ref: str):
    name = manifest["name"]
    result = {"type": "library", "bom-ref": ref}
    if name.startswith("@") and "/" in name:
        result["group"], result["name"] = name.split("/", 1)
    else:
        result["name"] = name
    result["version"] = manifest["version"]
    result["purl"] = ref
    license = declared_license(manifest)
    if license:
        result["licenses"] = [{"expression": license}]
    return result


def find_packages(roots):
    """Return a map from the real path of each package directory to its manifest."""
    packages = {}
    visited = set()
    directories = [Path(root) for root in roots]
    while directories:
        directory = directories.pop()
        real = Path(os.path.realpath(directory))
        if real in visited or not real.is_dir():
            continue
        visited.add(real)
        if is_package_directory(directory) or is_package_directory(real):
            manifest = read_manifest(real)
            if manifest is not None:
                packages[real] = manifest
        try:
            entries = sorted(real.iterdir())
        except OSError:
            continue
        directories.extend(entry for entry in entries if entry.is_dir())
    return packages


def resolve(directory: Path, name: str, packages):
    """Find the package `name` from `directory` like Node.js does."""
    for parent in [directory, *directory.parents]:
        if parent.name == "node_modules":
            continue
        candidate = Path(os.path.realpath(parent / "node_modules" / name))
        if candidate in packages:
            return candidate
    return None


def main():
    root_name, roots = sys.argv[1], sys.argv[2:]
    packages = find_packages(roots)
    refs = {path: purl(m["name"], m["version"]) for path, m in packages.items()}

    components = {}
    edges = {}
    for path, manifest in packages.items():
        ref = refs[path]
        components.setdefault(ref, component(manifest, ref))
        targets = edges.setdefault(ref, set())
        for field in ("dependencies", "optionalDependencies", "peerDependencies"):
            dependencies = manifest.get(field)
            if not isinstance(dependencies, dict):
                continue
            for name in dependencies:
                target = resolve(path, name, packages)
                if target is not None and refs[target] != ref:
                    targets.add(refs[target])

    # The packages that no other package depends on are the direct dependencies of the root.
    dependents = set().union(*edges.values())
    root_ref = "root"
    edges[root_ref] = set(edges) - dependents

    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": root_ref,
                "name": root_name,
            }
        },
        "components": [components[ref] for ref in sorted(components)],
        "dependencies": [
            {"ref": ref, "dependsOn": sorted(targets)}
            for ref, targets in sorted(edges.items())
        ],
    }
    json.dump(bom, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
