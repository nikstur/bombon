# Crane Rust Build SBOM Example

This is a basic example on how to use `bombon` with a `crane`-based rust build environment.

It's based on the [simple crane example](https://crane.dev/examples/quick-start-simple.html),
only adding the necessary `bombon` features to generate sboms.

It also adds a `regex` dependency and example code from the [cargo book](https://doc.rust-lang.org/cargo/guide/dependencies.html).

You can build the binary with:

```
$ nix build
```

You can build the SBOM with:

```
$ nix build .#sbom
```
