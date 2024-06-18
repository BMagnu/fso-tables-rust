# FSO Table Parsing in Rust

This crate aims to provide easy access for both parsing and spewing tables for the FreeSpace 2 Open engine.

## Basic Usage

At it's core, this crate aims to simplify table parsing to three steps:
1. Recreate the content of the table as a type in Rust.
2. Annotate the type with the ``#[fso_table]`` annotation.
3. Create a ``FSOTableFileParser`` object, and use it to create a populated version of the struct.

A very simple example:

```rust
#[fso_table(table_start="#Example", table_end="#End")]
pub struct SampleTable {
	pub name: String
}

fn parse() {
	let table = FSOTableFileParser::new(Path::new("/path/to/table.tbl")).and_then(|parser| SampleTable::parse(&parser));
	if let Ok(table) = table {
		//Do something
	}
}
```

The table specified by the ``ParseTable`` struct expects to start with ``#Example`` and end with ``#End``, while containing a single entry ``$Name: <string>``.
Often times, FSO's tables are more complex and contain nested structs and data.
For this purpose, the ``#[fso_table]`` annotation allows the struct to contain fields of structs that are themselves annotated with ``#[fso_table]``.

Spewing of tables is not yet implemented.

## Supported Field Types

Supported basic field types are the following:
``i32``, ``i64``, ``u32``, ``u64``, ``f32``, ``f64``, ``bool``, ``String``

Any struct annotated with ``#[fso_table]`` can also be used as a type in another ``#[fso_table]``.

Any enum annotated with ``#[fso_table]`` can similarly be used. Annotating an enum in such a way expects the next table key to be the name of any of the enum variants. If this is the case, the fields of the respective enum variant are parsed immediately after, seperated by spaces or commas. If more involved parsing behavior for the fields of the enum variant are needed, consider using a type from the list below or a struct annotated with ``#[fso_table]``.

Most basic types have fairly intuitive behavior. Some more complex types can alter table parsing behavior however:
- ``Box<T>``: Specifying a box will be transparent to the table, i.e. parsing will be identical to as if ``T`` had been specified directly. This can be used to manoeuvre difficult access or ownership structures.
- ``Rc<T>``, ``Arc<T>``, ``Cell<T>``, ``RefCell<T>``: Same as ``Box<T>``
- ``Vec<T>``: A vector will repeatedly parse the subtype ``T`` until this fails. It does _not_ require the name of the vector in the containing struct to be repeated. If such behavior is desired, create a vector of a ``#[fso_table]``-annotated struct that itself contains the desired name-value pair. The list can be optionally surrounded by parentheses. Individual elements can be seperated by spaces or commas. When the individual elements are ``#[fso_table]``-annotated structs, they can also be seperated by newlines. 
- ``Option<T>``: Denotes an optional key in the table. If the name is present, the value will be parsed. If it is not, parsing will be skipped for this value.
- ``(T...)``: Tuples will parse a list of inner keys that must be surrounded by parentheses in the table. These values can be seperated by spaces and commas.

If a type beyond the specified types is required to be parsed, it must manually implement the ``FSOTable`` trait.

## Table Modifiers

Certain annotations can be used to modify the table parsing behavior. One such example was shown above, where an annotation was used to require a specific table start and end token.
Available annotations are:

- ``required_parser_trait(trait...)``: Force the struct to require a parser struct that implements the specified types in ``trait...`` (specified as a comma seperated list). This is useful if you require a custom type to be parsed in the struct that itself requires more data (such as about states unrelated to this table) then is provided by the ``FSOParser`` trait.
- ``required_lifetime(lifetimes...)``: Useful in combination with the above, can be used to define (bounded) lifetimes, as you might require if you need to prove that the parsed table outlives the parser struct.

For annotated structs only:
- ``table_start="<value>"``: Requires a token ``<value>`` when parsing of this struct begins.
- ``table_end="<value>"``: Requires a token ``<value>`` when parsing of this struct ends.
- ``prefix="<value>"``: What is appended before the name of a field when parsing. Defaults to ``$``. Can be combined with the following to, for example, result in ``$<VariantName>:`` keys.
- ``suffix="<value>"``: What is appended before the name of a field when parsing. Defaults to ``:``.

For annotated enums only:
- ``prefix="<value>"``: What is appended before the name of an enum variant when parsing. Can be combined with the following to, for example, result in ``$<VariantName>:`` keys.
- ``suffix="<value>"``: What is appended before the name of an enum variant when parsing.
- ``flagset``: Converts all CamelCase enum variant names to spaced lower case as needed to parse flagsets.

## Field Modifiers:

In addition to the modifiers for the entire struct / enum, the individual fields of structs can be annotated as well.

For fields of structs:
- ``fso_name="<value>"``: Overrides the automatic generation of the key in the FSO table (which works out to ``$<capitalized name>:``) with ``value``.
- ``skip``: Skips parsing of this key, and hides it to the parsing and dumping functions. The type of any such field must implement ``Default``.
- ``unnamed``: Parses an unnamed value. Such a value is expected to not be preceded by a name, and to follow directly after the last parsed value (except whitespaces).
- ``gobble="<value>"``: Expects that after completely parsing the value, ``<value>`` is present in the table. This occurrence of ``<value>`` will be consumed before parsing the next value.
- ``existence``: Interprets the presence of a key with the given name at this point in the table as a value of ``true``. Can only be used for fields with the type ``bool``.

For variants of types:
- ``use_as_default_string``: Marks the last variant of the enum as the default case. If no prior variant matched, the current token will be stored as a ``String`` in the last enum variant instead of erroring.
- ``fso_name="<value>"``: As above. Is applied before ``prefix`` and ``suffix``.
