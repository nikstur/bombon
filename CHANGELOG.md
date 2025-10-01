# Changelog

## 0.4.0 (unreleased)

### Added

- Added the ability to extract patches from a derivation and include them in
  the SBOM.
- Added the ability to include multiple source URLs as external references.
- Added the ability to extract CPEs from Nix packages. "Guessed" CPEs in
  the `possibleCPEs` field are included as evidence in the SBOM.
- Added the deduplication of components based on PURL. On the first component
  with a certain PURL will be kept in the final SBOM.

### Fixed

- Fixed an issue where some components would receive an empty string as VCS
  external reference.

## 0.3.0

### Added

- Added the ability to collect SBOMs from vendored dependencies (e.g. from Rust
  or Go dependencies).
- Added the option `excludes` to `buildBom` to exclude store paths via regex
  patterns from the final SBOM.
- Added the option `extraPaths` to `buildBom` to consider extra dependencies
  but still generating an SBOM for the original derivation.
- Hashes of fixed output derivations are now included in the SBOM.
- A derivation's `src` url and hash are now included in the SBOM.
- Derivations' descriptions are now included in the SBOM.

### Changed

- `doc` and `man` outputs are not included in the SBOM anymore.
- Generate CycloneDX v1.5 SBOMs instead of v1.4.
- The created SBOMS are now reproducible because they derive their serial
  number from a known input instead of randomly generating it.

### Fixed

- Fixed cross-compilation for SBOMs.
