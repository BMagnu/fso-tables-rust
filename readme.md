# FSO Table Handling Library

This library contains two components:
1. Implementations of assorted FSO tables. For this, use the ``fso_tables_impl`` crate. As of 1.0, the following tables have implementations:
   - Animations (data only, without Moveables)
   - Curves (data and semantics)
2. Macros and structs to easily implement further FSO tables, with automatic generation of parsing and spewing methods. For this, use the ``fso_tables`` crate. Note that spewing tables is experimental as of 1.0. Similarly, support to parse and re-spew unknown table options is planned but not yet implemented.