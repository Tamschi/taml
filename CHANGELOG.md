# taml Changelog

<!-- markdownlint-disable no-trailing-punctuation -->

## 0.0.11

2021-08-01

* **Breaking:**
  * Added new `Position` trait that is required by certain functions now.
  * Quoted literals (data literals, strings, identifiers) now cannot contain [U+000D CARRIAGE RETURN (CR)](https://graphemica.com/000D)
    unless escaped as `\r`.

* Features:
  * The `Token::Newline` regex is now `\r?\n` (was `\n`).

## 0.0.10

2021-07-29

* **Breaking:**
  * `Decoded` is now `DataLiteral` and so on.
  * Renamed `Float` variants to `Decimal`
    > as these can very much be parsed as arbitrary-precision numbers, depending on the implementation.
  * Changed number literals to reject additional leading zeroes
    > as these could be mistaken for octal input.
  * Added additional lexer tokens `InvalidZeroPrefixedDecimal` and `InvalidZeroPrefixedInteger`,
    > which allow formatting and better error diagnostics involving these now-invalid tokens.

* Features:
  * `smartstring::validate` is now re-exported as `validate`.
    > You should call this to ensure the memory layout is correct!
  * Added diagnostics for (always invalid) zero-prefixed decimals and integers.

## 0.0.9

2021-07-15

* Features:
  * Added `DiagnosticType::UnknownEncoding` and `DiagnosticType::EncodeFailed`.

## 0.0.8

2021-07-12

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
