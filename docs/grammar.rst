TAML Grammar Reference
======================

Comments
--------

At (nearly) any point in the document, a line comment can be written as follows:

.. code-block:: taml

	// This is a comment. It stretches for the rest of the line.
	// This is another comment.

The only limitation to comment placement is the the line up to that point must be valid.
This may not be the case if a ``,`` or identifier is expected, or if a bracket is unmatched.

Line breaks
-----------

TAML does not use commas to delineate values, outside of inline lists and table rows. Instead, line breaks are a grammar token.

.. warning::

	"Line break" *specifically* and *exclusively* refers to Unicode code point `U+000A LINE FEED (LF) <https://graphemica.com/000A>`_.

Empty lines and lines containing only a comment always can be removed without changing the structure or contents of the document.

.. hint::

	``taml fmt`` preserves single empty lines but collapses longer blank parts of the document.

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
