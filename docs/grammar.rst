.. _grammar_reference:

.. TK: warn that if you create a new implementation of the format, you should test it against to to-be-created sample files.

TAML Grammar Reference
======================

.. Style Note: This page favours absolute precision over readability.

.. hint::

	This page is aimed at format support implementors.

	For a user manual (even when using a TAML library as developer), see :ref:`taml_by_example`.

TK: Use singular for headings.

All grammar is defined in terms of Unicode codepoint identity.

Where available, the canonical binary or at-rest encoding of TAML is UTF-8,
while its runtime text-API representation should use the canonical representation of arbitrary Unicode strings in the target ecosystem.

.. note::

	Where no standard Unicode text representation exists, it's likely best to provide only a binary UTF-8 API.

Whitespace
----------

.. note::

	TK: Format as regex section

	.. code-block:: regex

		[ \r\t]+

Whitespace is meaningless except when separating otherwise-joined tokens.

Note that `line breaks`_ are not included here.

.. _comments:

Comment
-------

.. note::

	TK: Format as regex section

	.. code-block:: regex

		//[^\r\n]+

At (nearly) any point in the document, a line comment can be written as follows:

.. code-block:: taml

	// This is a comment. It stretches for the rest of the line.
	// This is another comment.

The only limitation to comment placement is that the line up to that point must be otherwise complete.

.. _line breaks:

Line break
----------

TAML does not use commas to delineate values, outside of `inline lists`_ and rows_.

Instead, line breaks are a grammar token that separates comments_, headings_, `key-value pairs`_ and table_ rows_.

.. warning::

	"Line break" *specifically* and *exclusively* refers to Unicode code point `U+000A LINE FEED (LF) <https://graphemica.com/000A>`_.

	`U+000D CARRIAGE RETURN (CR) <https://graphemica.com/000D>`_ is an illegal character everywhere in TAML unless quoted.

Empty lines outside of quotes and lines containing only a comment always can be removed without changing the structure or contents of the document.

.. hint::

	``taml fmt`` preserves single empty lines but collapses longer blank parts of the document.

	``taml fix`` can fix your line endings for you without changing the meaning of quotes. (TODO)
	It warns about any occurrence of the character it doesn't fix by default, in either sense. (TODO)

Identifiers
-----------

.. note::

	TK: Format as regex section

	.. code-block:: regex

		[a-zA-Z_][a-zA-Z\-_0-9]*

	.. code-block:: regex

		`([^\\`]|\\\\|\\`)*`

Identifiers in TAML are arbitrary Unicode strings and can appear in two forms, verbatim and quoted:

Verbatim
^^^^^^^^

Verbatim identifiers must start with an ASCII-letter or underscore (``_``). They may contain only those codepoints plus ASCII digits and the hypen-minus character (``-``).

.. hint::

	Support for ``-`` is a compatibility affordance.

	When outlining a new configuration structure, I recommend for example ``a_b`` over ``a-b``, as the former is treated as single "word" by most text editors. (Try double-clicking each.)

Quoted
^^^^^^

Backtick (`````)-quoted identifiers are parsed as **completely arbitrary** Unicode strings.

Only the following characters are backlash-escaped:

- ``\`` as ``\\``
- ````` as ``\```

All other sequences starting with a backslash are invalid in quoted strings and *must* lead to an error.

.. warning::

	Identifiers formally may be empty or contain `U+0000 NULL <https://graphemica.com/0000>`_.

	However, parsers for ecosystems where this cannot be safely supported are free to limit support here, as long as this limitation is prominently declared.

	(A parser written in for example C# or Rust very much should support both, though. A parser written in C or C++ should consider not supporting NULL due to its common special meaning.)

	TK: Define an error code that should be used here. Something like TAML-L0001?

.. _key-value pairs:

Key
---

Only identifiers_ may be keys. Keys appear in section_ headers, enum variants_ and as part of key-value pairs like the following:

.. code-block:: taml

	key: value

(``value`` is a `unit variant`_ here, but could be replaced with any other value_.)

Value
-----

A value is any one of the following:

TK

.. warning::

	TAML processors should be as strict as at all sensible regarding value types.
	For example, if a string is expected, don't accept an integer and vice versa.

	In some cases, remapping TAML value types is a good idea, like when parsing `rust_decimal <https://crates.io/crates/rust-decimal>`_ values using `Serde <https://crates.io/crates/serde>`_, which should still be written as decimals_ in TAML but internally processed as strings. Such remappings should be done explicitly on a case-by-case basis.

Integer
-------

.. note::

	TK: Format as regex section

	.. code-block:: regex

		-?(0|[1-9]\d*)

A whole number with base 10.
Note that  ``-0`` is legal and *may* be interpreted differently from ``0``.

Additional leading zeroes are disallowed to avoid confusion with languages and/or parsing systems where this would denote base 8.

.. hint::

	If your configuration requires setting a bitfield, consider accepting it as data literal e.g. like this instead:

	.. code-block:: taml

		some_bitfield: <bits:1000_0001 1111_0000>
		another_encoding: <hex:81 F0>

Decimal
-------

.. note::

	TK: Format as regex section

	.. code-block:: regex

		-?(0|[1-9]\d*)\.\d+

A fractional base 10 number.
Note that  ``-0`` is legal and *may* be interpreted differently from ``0``.

Additional leading zeroes are disallowed for consistency with integers.
Additional trailing zeroes are considered idempotent and **must not make a difference when parsing a value**.

.. note::

	Integers and decimals *should* be considered disjoint.
	Don't accept one for the other unless not doing so would be unusually inconvenient.

.. note::

	Decimals, like integers, are not required to fit any particular binary representation.

	For example, they could be parsed and processed with arbitrary precision rather than as IEEE 754 float.

.. warning::

	``taml fmt`` removes idempotent trailing zeroes from decimals.

	``serde_taml`` excludes them while lexing, which also affects ``reserde``.

	Absolutely do not make any distinction regarding additional trailing zeroes in decimals when writing a lexer or parser.


.. _variants:

Enum Variants
-------------

TK

Unit Variant
^^^^^^^^^^^^

Unit variants are written as single identifiers_.

Notable unit variants are the boolean values ``true`` and ``false``, which are not associated with more specific grammar in TAML.

List
----

TK

Inline Lists
^^^^^^^^^^^^

.. _section:

Sections
--------

TAML's grammar is, roughly speaking, split into three contexts:

- structural sections
- headings
- tabular sections

Structural Sections
^^^^^^^^^^^^^^^^^^^

The initial context is a structural section.
Structural sections can contain key-value pairs and nested sections, which can be structural sections.

.. code-block:: taml

	first: 1
	second: 2

	# third
	first: 3.1
	second: 3.2

Each nested section is introduced by a heading nested *exactly* one deeper than the surrounding section's.

It continues until a heading with at most equal depth is encountered or up to the end of the file.
An empty nested heading can be used to semantically (but not grammatically!) return to its immediately surrounding structural section.

.. code-block:: taml

	first: 1
	second: 2

	# third
	first: 3.1
	second: 3.2

	## third
	first: "3.3.1"
	second: "3.3.2"

	## fourth
	first: "3.4.1"
	second: "3.4.2"

	#
	fourth: 4

Headings
^^^^^^^^

.. _table:

Tabular Sections
^^^^^^^^^^^^^^^^

Tabular sections are a special shorthand to quickly define lists with structured content.

The following are equivalent:

.. code-block:: taml

	# [[dishes].{id, name, [price].{currency, amount}]
	<luid:d6fce69d-9c9d>, "A", EUR, 10.95
	<luid:c37dcc6a-2002>, "B", EUR, 5.50
	<luid:00000000-0000>, "Test Item", EUR, 0.0

.. code-block:: taml

	# [dishes]
	id: <luid:d6fce69d-9c9d>
	name: "A"
	## price
	currency: EUR
	amount: 10.95

	# [dishes]
	id: <luid:c37dcc6a-2002>
	name: "B"
	## price
	currency: EUR
	amount: 5.50

	# [dishes]
	id: <luid:00000000-0000>
	name: "Test Item"
	## price
	currency: EUR
	amount: 0.0

.. hint::

	As of right now, there is intentionally no way to define common values once per table.

	I haven't found a way to express this that both is intuitive and won't make copy/paste errors much more likely.

.. _rows:

Row
"""

TK
