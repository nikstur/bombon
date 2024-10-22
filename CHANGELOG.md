# Changelog

## 0.3.0 (unreleased)

- `doc` and `man` outputs are not included in the SBOM anymore.
- Added the option `excludes` to `buildBom` to exclude store paths via regex
  patterns from the final SBOM.
- Generate CycloneDX v1.5 SBOMs instead of v1.4.
- Hashes of fixed output derivations are now included in the SBOM.
