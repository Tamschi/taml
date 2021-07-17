.. TAML documentation master file, created by
   sphinx-quickstart on Sat Jul 17 14:59:37 2021.
   You can adapt this file completely to your liking, but it should at least
   contain the root `toctree` directive.

TAML Reference Documentation
============================

TAML is a configuration file format combining some aspects of Markdown, CSV, TOML, YAML and Rust.

As configuration language, TAML's main design goals are to be:

- Human-writeable
- Human-readable
- Unambiguous and Debuggable
- Computer-readable

One central feature is that it uses headings rather than indentation or far-spanning nested brackets to denote complex data structures.
Another is the relatively strong distinction between data types.

In addition to this, implementations of the file format *should* make it easy to make it easy for software end(!) users to learn about and correct mistakes in configuration files.
A number of error codes and descriptions are documented here which will ideally be largely shared between implementations.

Please refer to the table of contents to the left for examples and details.

.. note::

	TAML is explicitly not a data transfer format.

	Most notably, it is **not streamable**, as repeated fields are not valid and *must* lead to a parsing error.

.. toctree::
   :maxdepth: 2
   :caption: Contents:

	grammar
	diagnostics
	formatting



.. Indices and tables
.. ==================

.. * :ref:`genindex`
.. * :ref:`modindex`
.. * :ref:`search`
