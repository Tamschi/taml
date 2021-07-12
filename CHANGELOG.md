# taml Changelog

<!-- markdownlint-disable no-trailing-punctuation -->

## next

TODO: Date

* Fixed:
  * The span info inside `Decoded` was calculated really wrong during lexing.

## 0.0.7

2021-07-12

* **Breaking:**
  * Extended `Token::Decoded` and `TamlValue::Decoded` to carry additional span information.

    > This also makes `Token` generic, but only `Token<usize>` implements `logos::Lexer`.

## 0.0.6

2021-07-12

* **Breaking:**
  * Added `<encoding:Decoded \>text>` buffer strings to the grammar.

    These are exposed (without processing beyond the initial unescape!) via the `Token::Decoded((…, …))` and `TamlValue::Decoded { … }` variants.

## 0.0.5

2021-07-05

* Features:
  * Implemented `Borrow<str>` for `Key`.

## 0.0.4

2021-07-03

* **Breaking:**
  * Increased minimum Rust version from 1.46 to 1.53
    > to use nested or-patterns, which simplifies the parser code somewhat.
  * Increased version of `logos` dependency from `"0.11.4"` to `"0.12.4"`
    > to move away from a faulty version of the `beef` crate further upstream.

## 0.0.3

2020-09-22

* Ignore meta data tests when running with Miri
* Expanded contribution guidelines
* Public dependency upgrades:
  * `cervine`: 0.0.5 to 0.0.6

## 0.0.2

2020-09-11

* Added meta data
* Started on documentation
* Public dependency upgrades:
  * `cervine`: 0.0.2 to 0.0.5

## 0.0.1

2020-08-26

Initial unstable release
