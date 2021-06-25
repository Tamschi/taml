# TAML

[![Lib.rs](https://img.shields.io/badge/Lib.rs-*-84f)](https://lib.rs/crates/taml)
[![Crates.io](https://img.shields.io/crates/v/taml)](https://crates.io/crates/taml)
[![Docs.rs](https://docs.rs/taml/badge.svg)](https://docs.rs/taml)

![Rust 1.53](https://img.shields.io/static/v1?logo=Rust&label=&message=1.53&color=grey)
[![CI](https://github.com/Tamschi/taml/workflows/CI/badge.svg?branch=develop)](https://github.com/Tamschi/taml/actions?query=workflow%3ACI+branch%3Adevelop)
![Crates.io - License](https://img.shields.io/crates/l/taml/0.0.3)

[![GitHub](https://img.shields.io/static/v1?logo=GitHub&label=&message=%20&color=grey)](https://github.com/Tamschi/taml)
[![open issues](https://img.shields.io/github/issues-raw/Tamschi/taml)](https://github.com/Tamschi/taml/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/Tamschi/taml)](https://github.com/Tamschi/taml/pulls)
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

## [Code of Conduct](CODE_OF_CONDUCT.md)

## [Changelog](CHANGELOG.md)

## The grammar

TAML (always UTF-8 where applicable) can represent much of the [Serde data model](https://serde.rs/data-model.html) verbatim and is self-describing, as follows:

- [`bool`](https://doc.rust-lang.org/stable/std/primitive.bool.html) is represented as enum with the unit values `true` and `false`,
- all integers are base 10 with optional preceding `-`.
  - You can (try to) deserialize them as any primitive Rust integer.
  - When deserialized as any, the result is the smallest fitting unsigned or signed integer type.
- all floats are base 10 with infix `.` (meaning at least one leading and trailing digit is necessary, each).
  - Deserializing as any results in an f64.
- [`char`](https://doc.rust-lang.org/stable/std/primitive.char.html) matches Rust: `'🦀'`
- **byte array**s are unsupported,
- **option**s are flattened.
  - This means only the [`Some(_)`](https://doc.rust-lang.org/stable/std/option/enum.Option.html#variant.Some)-variant can be present in **seq**s.
  - Use `#[serde(default)]` to parse missing **struct** keys as [`None`](https://doc.rust-lang.org/stable/std/option/enum.Option.html#variant.None).
- [`unit`](https://doc.rust-lang.org/stable/std/primitive.unit.html) and **unit struct**s are written as **empty seq**.
- a **unit variant** is written as the variant key: `Yes`
- **newtype struct**s are flattened,
- **newtype variant**s are written as key followed by a **seq**: `No("impossible!")`
- **seq**:
  - either inline (*in a single line*) similar to Rust tuples:

    ```taml
    ((a, b, c), (d, e)) // seqs can be directly nested this way, but then cannot contain structures.
    ```

  - in heading paths:

    ```taml
    # path.with.a.[list].inside
    ```

  - or as tabular section:

    ```taml
    # path.to.[[list]]
    1
    2
    3
    4
    "five"

    # [[vectors].{a, b, c, d}]
    1.0, 2.0, 3.0, 4.0
    1.1, 2.1, 3.1, 4.1
    2.1, 2.2, 3.2, 4.2
    3.1, 3.2, 3.3, 4.3
    4.1, 4.2, 4.3, 4.4
    ```

- **tuple** and **tuple_struct**: as **seq**, but with length validation
- **tuple_variant**: as key directly followed by an inline **seq**, with length validation
- **map**s, **struct**s and **struct_variant**s are written as sections containing key-value-pairs (one per line), possibly with subsections.
  - Example:

    ```taml
    # path.to.structure
    key: Value
    `another key`: 1.0

    ## structured_variant:VariantKey
    `nested\`field`: "nested`field"
    ```

  - Unlike most Serde formats, **TAML by default errors out on unexpected *struct* or *struct_variant* fields!**

    However, you can collect them in a **map** by adding a field with the name `"taml::extra_fields"`. Use [`#[serde(rename = "taml::extra_fields")]`](https://serde.rs/field-attrs.html#rename) or equivalent.

<!--
If you intend to write a custom parser for this format, please validate it against the sample files in `tests/samples`. (TODO: Create those.)
-->

TODO: Describe headings.

## Versioning

`taml` strictly follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html) with the following exceptions:

- The minor version will not reset to 0 on major version changes (except for v1).  
Consider it the global feature level.
- The patch version will not reset to 0 on major or minor version changes (except for v0.1 and v1).  
Consider it the global patch level.

This includes the Rust version requirement specified above.  
Earlier Rust versions may be compatible, but this can change with minor or patch releases.

Which versions are affected by features and patches can be determined from the respective headings in [CHANGELOG.md](CHANGELOG.md).
