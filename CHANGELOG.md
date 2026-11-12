# Changelog

## Unreleased

### Added

- Added `passthruVendoredSbom.npm` to generate SBOMs for vendored npm
  dependencies of packages that use `npmConfigHook` (e.g. anything built with
  `buildNpmPackage`). The SBOM is generated with `npm sbom`.
- Added `passthruVendoredSbom.pnpm` to generate SBOMs for vendored pnpm
  dependencies of packages that use `pnpmConfigHook`. The SBOM is generated
  with `pnpm sbom`, which requires pnpm 11 or newer.
- Added `passthruVendoredSbom.go` to generate SBOMs for vendored Go
  dependencies of packages built with `buildGoModule`. The SBOM is generated
  with `cyclonedx-gomod`.
- Added emitting a CycloneDX dependency graph spanning runtime dependencies,
  vendored language-level SBOMs' graphs, and optionally build-time dependencies
  via the `includeBuildtimeDependencies` flag.
- Added support for compound licenses from Nixpkgs. Compound licenses are now
  included as SPDX expressions in the SBOM.
- Added support for CycloneDX v1.7.

### Fixed

- Fixed `passthruVendoredSbom.pnpm` for packages that use their pnpm store
  directly from the read-only Nix store. `pnpm sbom` opens the store index
  read-write, so it failed with `attempt to write a readonly database`.
- Fixed `excludes` for patterns with characters that are special to the shell,
  like `(`, `|`, `*` or a space, and for patterns that begin with a dash. The
  patterns were not quoted, so building the SBOM failed.
- Fixed building an SBOM realising the whole build closure of the package,
  even without `includeBuildtimeDependencies`. Only the patches and vendored
  SBOMs that the transformer reads are now built or substituted.
- Fixed dependencies being lost when components are deduplicated by their
  PURL. The dependencies of a removed duplicate (e.g. another output of the
  same derivation) and the dependencies on it now belong to the component kept
  in its place. Of the duplicates, a store path from the runtime closure is
  kept, then the `out` output, then one with evaluated metadata.
- Fixed build-time dependencies being deduplicated by their name with
  `includeBuildtimeDependencies`. All outputs of a derivation and the stages
  of the bootstrap share a name, so only one arbitrary store path per name was
  kept (e.g. only the `dev` output of curl) and the dependencies on the others
  were lost. They are now deduplicated like the runtime dependencies.
- Fixed dependencies being lost in the dependency graph when they go through a
  store path that is not a component (e.g. one without a version, a `doc` or
  `man` output, or one matched by `excludes`). A component now depends on
  every component it refers to directly or only through such store paths at
  runtime. At build time, it only depends on such a component through these
  store paths if it doesn't reach it otherwise.

## 0.4.0

### Added

- Added the ability to extract patches from a derivation and include them in
  the SBOM.
- Added the ability to include multiple source URLs as external references.
- Added the ability to extract CPEs from Nix packages. "Guessed" CPEs in
  the `possibleCPEs` field are included as evidence in the SBOM.
- Added deduplication of components based on PURL. Only the first component
  with a certain PURL will be kept in the final SBOM.

### Changed

- Derivations without a version are now excluded from the final SBOM as these
  are usually ad-hoc created (e.g. for systemd units, etc.) and thus not
  relevant.
- Improved the guessing of versions from a store path. Now the last component
  is usually picked up as the version of the package.

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
