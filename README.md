# TAML

[![Lib.rs](https://img.shields.io/badge/Lib.rs-*-84f)](https://lib.rs/crates/taml)
[![Crates.io](https://img.shields.io/crates/v/taml)](https://crates.io/crates/taml)
[![Docs.rs](https://docs.rs/taml/badge.svg)](https://docs.rs/taml)

![Rust 1.53](https://img.shields.io/static/v1?logo=Rust&label=&message=1.53&color=grey)
[![CI](https://github.com/Tamschi/taml/workflows/CI/badge.svg?branch=develop)](https://github.com/Tamschi/taml/actions?query=workflow%3ACI+branch%3Adevelop)
![Crates.io - License](https://img.shields.io/crates/l/taml/0.0.11)

[![GitHub](https://img.shields.io/static/v1?logo=GitHub&label=&message=%20&color=grey)](https://github.com/Tamschi/taml)
[![open issues](https://img.shields.io/github/issues-raw/Tamschi/taml)](https://github.com/Tamschi/taml/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/Tamschi/taml)](https://github.com/Tamschi/taml/pulls)
[![good first issues](https://img.shields.io/github/issues-raw/Tamschi/taml/good%20first%20issue?label=good+first+issues)](https://github.com/Tamschi/taml/contribute)

[![crev reviews](https://web.crev.dev/rust-reviews/badge/crev_count/taml.svg)](https://web.crev.dev/rust-reviews/crate/taml/)

TAML is a configuration file format combining some aspects of Markdown, CSV, TOML, YAML and Rust.

As configuration language, TAML's main design goals are to be:

- Human-writeable
- Human-readable
- Unambiguous and Debuggable
- Computer-readable

Since it is mainly human-oriented and the same data can be represented in multiple ways, there is in fact no serializer in this library. If you need a data transfer format, pretty much anything else will give you better performance.

That said, I believe that for human-written files, TAML offers a great balance between brevity and simplicity, with more than sufficient performance.

A command line validator and formatter is available in the [`taml-cli`] crate.  
Serde-intergration can be found in [`serde_taml`].

See <https://taml.schichler.dev> for documentation on the format itself.

[`taml-cli`]: https://github.com/Tamschi/taml-cli
[`serde_taml`]: https://github.com/Tamschi/serde_taml/

## Installation

Please use [cargo-edit](https://crates.io/crates/cargo-edit) to always add the latest version of this library:

```cmd
cargo add taml
```

## Example

TODO: Add a good example file here.

## License

Licensed under either of

- Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING](CONTRIBUTING.md) for more information.

## [Code of Conduct](CODE_OF_CONDUCT.md)

## [Changelog](CHANGELOG.md)

## Versioning

`taml` strictly follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html) with the following exceptions:

- **Invalid TAML becoming valid is considered a feature addition.**
- The minor version will not reset to 0 on major version changes (except for v1).  
Consider it the global feature level.
- The patch version will not reset to 0 on major or minor version changes (except for v0.1 and v1).  
Consider it the global patch level.

This includes the Rust version requirement specified above.  
Earlier Rust versions may be compatible, but this can change with minor or patch releases.

Which versions are affected by features and patches can be determined from the respective headings in [CHANGELOG.md](CHANGELOG.md).
