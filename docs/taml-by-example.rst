.. _taml_by_example:

TAML by Example
===============

.. Style Note: This page favours readability over absolute precision.

.. hint::

	This is a non-normative quickstart guide.

	For the thorough format reference, see :ref:`grammar_reference`.

The most simple TAML document is empty:

.. code-block:: taml

While this is valid TAML, such a configuration file will usually be unsupported in practice, as fields are not always optional.

Key-Value Pairs
---------------

To define structural fields (including in the implicit top-level context), you can write them as key-value pairs as follows:

.. code-block:: taml

	// This is a comment. The parser will ignore it.
	a_string: "This is Unicode text. You can escape \\ and \"."
	some_data: <Some-Encoding:This is a data literal. You can escape \\ and \>.>
	an_integer: 5
	negative: -0
	decimal: 0.0
	negative_decimal: -10.0
	list: ("Inline lists may contain heterogeneous data but no line breaks.", 1, 2.0, ())

	`You can quote identifiers and escape \\ and \` within.`: ()

.. note::

	Integers and decimals are disjoint! If a decimal value is expected, write ``1.0`` instead of ``1``.

Tabular List
------------

Lists may also be written as tables with a single column:

.. code-block:: taml

	# [[items]]
	"This is a list in tabular form."

	1
	2
	3
	4
	5

	"This is still part of the list."

Lists continue up to the next heading or end of document. They may not contain nested sections.

Each row in the table creates a new column in the list assigned to the ``items`` field,
while empty lines and lines with only a comment are ignored.

Structural Section
------------------

Complex data structures can be represented in TAML as follows:

.. code-block:: taml

	top_level_field: ()

	# outer_structural_field
	inner_field: ()

	## inner_structural_field
	deeply_nested: ()

	#
	another_top_level_field: ()

This is equivalent to the following JSON:

.. code-block:: json

	{
		"top_level_field": [],
		"outer_structural_field": {
			"inner_field": [],
			"inner_structural_field": {
				"deeply_nested": []
			}
		},
		"another_top_level_field": []
	}

Structures in Lists
-------------------

Structure headings create list items whenever identifiers are wrapped in square brackets (``[…]``):

.. code-block:: taml

	# [items]
	a: 1
	b: 2

	# [items]
	a: 3
	b: 4
	c: 5

equals

.. code-block:: json

	"items": [
		{
			"a": 1,
			"b": 2
		},
		{
			"a": 3,
			"b": 4,
			"c": 5
		}
	]

.. note::

	Fields that are defined twice are normally invalid. However, adding items to an existing list is possible as above.

Path Heading
------------

The following are equivalent:

.. code-block:: taml

	# a
	## [b]
	### c
	d: 1
	e: 2

	## f
	### g
	#### [h]
	##### [[j]]
	1
	2
	3
	4
	5

	# k
	## l
	### m
	### n

	// Illegal, would redefine `a`:
	// # a
	// ## o

.. code-block:: taml

	# a
	## [b].c
	d: 1
	e: 2

	## f.g.[h].[[j]]
	1
	2
	3
	4
	5

	# k.l
	## m
	## n

	// Illegal, would redefine `a`:
	// # a.o

Multi-Column Table
------------------

The following are equivalent:

.. code-block:: taml

	# [a]
	b: 1
	## [c]
	## d
	e: 2
	f: 3
	##
	g: 4

	# [a]
	b: 5
	## [[c]]
	6
	7
	## d
	e: 8
	f: 9
	##
	g: 10

.. code-block:: taml

	# [[a].{b, c, d.{e, f}, g}]
	1, (), 2, 3, 4
	5, (6, 7), 8, 9, 10

.. hint::

	I don't recommend manually aligning table cells here, as some people (including me) use proportional fonts almost everywhere.

	(``taml fmt`` would undo it by default, too.)

.. hint::

	You can write ``.{}`` in a table heading to assign an empty structure to a field in each row.

Or as JSON:

.. code-block:: json

	{
		"a": [
			{
				"b": 1,
				"c": [],
				"d": {
					"e": 2,
					"f": 3
				},
				"g": 4
			},
			{
				"b": 5,
				"c": [
					6,
					7
				],
				"d": {
					"e": 8,
					"f": 9
				},
				"g": 10
			}
		]
	}