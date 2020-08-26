# TAML¹

```taml
summary: "TAML is a configuration file format combining some aspects of Markdown, CSV, TOML, YAML and Rust."

# [[credits].{handle, role, description}]
"Tamschi", Programming, "Looked at some JSON snippet and thought \"restating those keys is a grind 😒\"."
"TheBerkin", Naming, "Came up with the acronym, and the backronym shortly thereafter."

# changelog

## [[next]]
"Initial release"
```

As configuration language, TAML's main design goals are to be:

- Human-writeable
- Human-readable
- Unambiguous and Debuggable
- Computer-readable

Since it is mainly human-oriented and the same data can be represented in multiple ways, and some validation applied during parsing prevents full Serde-streaming, there is in fact no serializer in this library. If you need a data transfer format, pretty much anything else will give you better performance.

That said, I believe that for human-written files, TAML offers a great balance between brevity and simplicity, with more than sufficient performance.

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
- [`unit`](https://doc.rust-lang.org/stable/std/primitive.unit.html) and **unit_struct**s are written as **empty seq**.
- a **unit_variant** is written as the variant key: `Yes`
- **newtype_struct**s are flattened,
- **vewtype_variant**s are written as key followed by a **seq**: `No("impossible!")`
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

¹  Feel free to call it **T**amme's **A**mazing² **M**arkup **L**anguage.

²  You can drop this word. It fortunately works out either way.
