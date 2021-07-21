TAML Grammar Reference
======================

Whitespace
----------

.. hint::

	TK: Format as regex section

	.. code-block:: regex

		[ \r\t]+

Whitespace is meaningless except when separating otherwise-joined tokens.

Note that `line breaks`_ are not included here.

Comments
--------

.. hint::

	TK: Format as regex section

	.. code-block:: regex

		//[^\r\n]+

At (nearly) any point in the document, a line comment can be written as follows:

.. code-block:: taml

	// This is a comment. It stretches for the rest of the line.
	// This is another comment.

The only limitation to comment placement is that the line up to that point must be valid.
This may not be the case if a ``,`` or identifier is expected, or if a bracket is unmatched.

Line breaks
-----------

TAML does not use commas to delineate values, outside of inline lists and table rows. Instead, line breaks are a grammar token.

.. warning::

	"Line break" *specifically* and *exclusively* refers to Unicode code point `U+000A LINE FEED (LF) <https://graphemica.com/000A>`_.

	`U+000D CARRIAGE RETURN (CR) <https://graphemica.com/000D>`_ is an illegal character everywhere in TAML unless quoted.

Empty lines and lines containing only a comment always can be removed without changing the structure or contents of the document.

.. hint::

	``taml fmt`` preserves single empty lines but collapses longer blank parts of the document.

Identifiers
-----------

.. hint::

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

Keys
----

Only identifiers_ may be keys. Keys appear in section_ headers, enum variants_ and as part of key-value pairs like the following:

.. code-block:: taml

	key: value

(``value`` is a `unit variant`_ here, but could be replaced with any other value_.)

.. _value:

Values
------

A value is any one of the following:

TK

.. warning::

	TAML processors should be as strict as at all sensible regarding value types.
	For example, if a string is expected, don't accept an integer and vice versa.

	In some cases, remapping TAML value types is a good idea, like when parsing `rust_decimal <https://crates.io/crates/rust-decimal>`_ values using `Serde <https://crates.io/crates/serde>`_, which should still be written as decimals_ in TAML but internally processed as strings. Such remappings should be done explicitly on a case-by-case basis.

Decimals
--------

TK

.. _variants:

Enum Variants
-------------

TK

Unit Variant
^^^^^^^^^^^^

Unit variants are written as single identifiers_.

Notable unit variants are the boolean values ``true`` and ``false``, which are not associated with more specific grammar in TAML.

.. _section:

Sections
--------

TAML's grammar is, roughly speaking, split into three contexts:

- structure sections
- headings
- tabular sections

The initial context is a structure section.
Structure sections can contain key-value pairs and nested sections, which can be structure sections.

.. code-block:: taml

	first: 1
	second: 2

	# third
	first: 3.1
	second: 3.2

Each nested section is introduced by a heading nested *exactly* one deeper than the surrounding section's.

It continues until a heading with at most equal depth is encountered or up to the end of the file. An empty nested heading can be used to semantically (but not grammatically!) return to its immediately surrounding structure section.

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
