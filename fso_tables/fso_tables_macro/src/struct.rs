use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use regex::Regex;
use syn::{Error, Expr, ExprLit, Field, ItemStruct, Lit, Meta, MetaNameValue, Type};
use syn::parse::Parser;
use syn::spanned::Spanned;
use crate::typehandler::{deduce_type, FSONaming, FSOValueType};
use crate::util::{fso_build_impl_generics, fso_build_where_clause};

pub(crate) struct TableField {
	fso_name: FSONaming,
	fso_gobble: Option<String>,
	rust_token: Ident,
	rust_type: Type,
	rust_span: Span,
	field_number: usize
}

pub(crate) fn fso_struct_build_parse(fields: &Vec<TableField>, inline: bool) -> Result<(TokenStream, TokenStream, TokenStream), Error> {
	let mut parse = quote! ();
	let mut fill = TokenStream::new();
	let mut spew = TokenStream::new();
	
	let or_else_fail = quote! {.map_err(|mut err: fso_tables::FSOParsingError| {
		err.comments = __comment.clone();
		err.version_string = __version_string.clone();
		err
	})?};

	for field in fields.iter() {
		let name = &field.rust_token;

		if let FSONaming::Skipped = field.fso_name {
			let typename = &field.rust_type;
			fill = quote!(
				#fill
				#name: #typename::default(),
			);
			continue;
		}

		let parse_comments;
		let process_comments;
		if inline {
			parse_comments = quote!{};
			process_comments = quote!{};
		}
		else {
			let field_num = field.field_number;
			parse_comments = quote!{
				if !__already_parsed_comments {
					(__comment, __version_string) = state.consume_whitespace(false);
					__already_parsed_comments = true;
				}	
			};
			process_comments = quote!{
				__already_parsed_comments = false;
				__comments[#field_num] = __comment.clone();
				__version_strings[#field_num] = __version_string.clone();
			};
		}

		let parse_gobble = if let Some(gobble) = &field.fso_gobble {
			quote! {
				state.consume_whitespace_inline(&[]);
				state.consume_string(#gobble)#or_else_fail;
			}
		}
		else {
			quote!()
		};
		let spew_gobble = if let Some(gobble) = &field.fso_gobble {
			quote! {
				state.append(#gobble);
				state.append(" ");
			}
		}
		else {
			quote!(state.append(" ");)
		};

		let parse_inner_gobble = quote!{
			if let Some(__inner_gobble) = __inner_gobble {
				__comment = __inner_gobble.comments;
				__version_string = __inner_gobble.version_string;
				__already_parsed_comments = true;
			}
		};
		
		let (value_type, make_type, spew_type) = deduce_type(&field.fso_name, &field.rust_type, &format_ident!("__to_spew"), &format_ident!("None"))?;
		let (parse_value, spew_value) = match &field.fso_name {
			FSONaming::Named { fso_name, .. } => {
				match value_type {
					FSOValueType::Option { .. } => {
						(quote!{
							let #name = if let Ok(_) = state.consume_string(#fso_name) {
								#process_comments
								let (__opt_result, __inner_gobble) = #make_type #or_else_fail; //Named Optionals must be parseable
								#parse_inner_gobble
								#parse_gobble
								Some(__opt_result)
							}
							else {
								None
							};
						},
						quote!{
							if let Some(__to_spew) = &self.#name {
								state.append("\n");
								state.append(#fso_name);
								state.append(" ");
								#spew_type
								#spew_gobble
							}
						})
					}
					_ => {
						(quote!{
							state.consume_string(#fso_name)#or_else_fail;
							#process_comments
							let (#name, __inner_gobble) = #make_type #or_else_fail;
							#parse_inner_gobble
							#parse_gobble
						},
						quote!{
							{
								let __to_spew = &self.#name;
								state.append("\n");
								state.append(#fso_name);
								state.append(" ");
								#spew_type
								#spew_gobble
							}
						})
					}
				}
			}
			FSONaming::Unnamed => {
				match value_type {
					FSOValueType::Option { .. } => {
						(quote!{
							let #name = if let Ok((data, __inner_gobble)) = #make_type { //Unnamed Optionals can fail during parsing itself, that's assumed to be "non-existant"
								#process_comments
								#parse_inner_gobble
								#parse_gobble
								Some(data)
							}
							else {
								None
							};
						},
						quote!{
							if let Some(__to_spew) = &self.#name {
								#spew_type
								#spew_gobble
							}
						})
					}
					_ => {
						(quote!{
							#process_comments
							let (#name, __inner_gobble) = #make_type #or_else_fail;
							#parse_inner_gobble
							#parse_gobble
						},
						quote!{
							{
								let __to_spew = &self.#name;
								#spew_type
								#spew_gobble
							}
						})
					}
				}
			}
			FSONaming::ExistenceIsBool { fso_name } => {
				match value_type {
					FSOValueType::Direct { ty: Type::Path( path ) } if path.path.is_ident("bool") => {
						(quote!{
							let #name = state.consume_string(#fso_name).is_ok();
							if #name {
								#process_comments
								#parse_gobble
							}
						},
						quote! {
							if self.#name {
								state.append("\n");
								state.append(#fso_name);
								#spew_gobble
							}
						})
					}
					_ => {
						return Err(Error::new(field.rust_span, "Only variables of type bool can be existence-bool'd!"));
					}
				}
			}
			FSONaming::Skipped => { unreachable!() }
		};

		parse = quote!(
			#parse
			#parse_comments
			#parse_value
		);

		fill = quote!(
			#fill
			#name,
		);

		spew = quote!(
			#spew
			#spew_value
		);
	}

	Ok((parse, fill, spew))
}

pub(crate) fn fso_table_struct(item_struct: &mut ItemStruct, instancing_req: Vec<TokenStream>, lifetime_req: Vec<TokenStream>, table_prefix: Option<String>, table_suffix: Option<String>, prefix: Option<String>, suffix: Option<String>, inline: bool) -> Result<(TokenStream, TokenStream), Error> {
	let mut table_fields: Vec<TableField> = Vec::new();
	let struct_name = &item_struct.ident;
	let (_, ty_generics, where_clause) = item_struct.generics.split_for_impl();
	let mut field_count: usize = if table_prefix.is_some() { 1 } else { 0 };
	let mut field_comma_list = quote!();
	
	if let syn::Fields::Named(ref mut fields) = item_struct.fields {
		for field in fields.named.iter_mut() {
			let rust_type = field.ty.clone();
			let forced_table_name = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::NameValue( MetaNameValue { value: Expr::Lit( ExprLit{ lit: Lit::Str(new_name), ..}), .. })
				if a.meta.path().is_ident("fso_name") => { Some(Ok(new_name.value())) },
				_ if a.meta.path().is_ident("fso_name") => {
					return Some(Err(Error::new(a.span(), "Attribute fso_name must have a value!")));
				}
				_ => { None }
			});
			let fso_gobble = if let Some(gobble) = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::NameValue( MetaNameValue { value: Expr::Lit( ExprLit{ lit: Lit::Str(new_name), ..}), .. })
				if a.meta.path().is_ident("gobble") => { Some(Ok(new_name.value())) },
				_ if a.meta.path().is_ident("gobble") => {
					return Some(Err(Error::new(a.span(), "Attribute gobble must have a value!")));
				}
				_ => { None }
			}) {
				Some(gobble?)
			}
			else {
				None
			};

			let skip = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::Path( path ) if path.is_ident("skip") => {
					Some(())
				}
				_ => { None }
			});
			let unnamed = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::Path( path ) if path.is_ident("unnamed") => {
					Some(())
				}
				_ => { None }
			});
			let existence_is_bool = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::Path( path ) if path.is_ident("existence") => {
					Some(())
				}
				_ => { None }
			});
			let multiline = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::Path( path ) if path.is_ident("multiline") => {
					Some(())
				}
				_ => { None }
			});
			
			field.attrs.retain(|a| !(
				a.path().is_ident("fso_name") ||
				a.path().is_ident("gobble") ||
				a.path().is_ident("skip") ||
				a.path().is_ident("unnamed") ||
				a.path().is_ident("existence") || 
				a.path().is_ident("multiline")));

			if let Some(ident) = field.ident.as_ref() {
				let rust_token = ident.to_string();
				let fso_name;

				if skip.is_some() {
					fso_name = FSONaming::Skipped;
				}
				else if unnamed.is_none() {
					thread_local! { static UNDERSCORE_REGEX: Regex = Regex::new(r"([^_]+)").unwrap(); }

					let mut out_name = "".to_string();
					UNDERSCORE_REGEX.with(|regex| {
						for (number, name_part) in regex.find_iter(&rust_token).enumerate() {
							out_name = format!("{}{}{}{}", out_name, if number == 0 { "" } else { " " }, name_part.as_str()[..1].to_uppercase(), &name_part.as_str()[1..]);
						}
					});
					
					let default_suffix = if existence_is_bool.is_none() { ":" } else { "" }.to_string();
					let actual_name = forced_table_name.unwrap_or(
						Ok(format!("{}{}{}", prefix.clone().unwrap_or("$".to_string()), out_name, suffix.clone().unwrap_or(default_suffix)))
					)?;
					if existence_is_bool.is_none() {
						fso_name = FSONaming::Named { fso_name: actual_name, multiline: multiline.is_some() };
					}
					else {
						fso_name = FSONaming::ExistenceIsBool { fso_name: actual_name };
					}
				}
				else {
					fso_name = FSONaming::Unnamed;
				}

				field_comma_list = quote!{
					#field_comma_list #ident: #rust_type,
				};
				
				table_fields.push(TableField { fso_name, fso_gobble, rust_token: ident.clone(), rust_type, rust_span: field.span(), field_number: field_count });
				field_count += 1;
			}
		}

		field_count += if table_prefix.is_some() { 1 } else { 0 };
		
		fields.named.push(Field::parse_named.parse2(quote! { __comments: [Option<String>; #field_count] })?);
		fields.named.push(Field::parse_named.parse2(quote! { __version_strings: [Option<String>; #field_count] })?);
	}
	else {
		return Err(Error::new(item_struct.fields.span(), "A struct annotated with fso_table must have named fields!"));
	}

	let impl_with_generics = fso_build_impl_generics(&lifetime_req, &item_struct.generics);

	let where_clause_with_parser = fso_build_where_clause(&instancing_req, &where_clause);

	let (parser, filler, spew) = fso_struct_build_parse(&table_fields, inline)?;

	let (prefix_parser, prefix_spewer) = if let Some(prefix) = table_prefix{
		(quote! {
			if !__already_parsed_comments {
				(__comment, __version_string) = state.consume_whitespace(false);
			}
			__comments[0] = __comment.clone();
			__version_strings[0] = __version_string.clone();
			__already_parsed_comments = false;
			state.consume_string(#prefix).map_err(|mut err: fso_tables::FSOParsingError| {
				err.comments = __comment.clone();
				err.version_string = __version_string.clone();
				err
			})?;
		}, quote! {
			state.append(#prefix);
			state.append("\n\n");
		})
	}
	else {
		(quote!{}, quote!{})
	};
	let (suffix_parser, suffix_spewer) = if let Some(suffix) = table_suffix{
		let suffix_field = field_count - 1;
		(quote! {
			if !__already_parsed_comments {
				(__comment, __version_string) = state.consume_whitespace(false);
			}
			__comments[#suffix_field] = __comment.clone();
			__version_strings[#suffix_field] = __version_string.clone();
			__already_parsed_comments = true;
			state.consume_string(#suffix)?;
		}, quote! {
			state.append("\n\n");
			state.append(#suffix);
		})
	}
	else {
		(quote!{}, quote!{})
	};

	Ok((quote! {
		impl fso_tables::FSOTable for #struct_name #ty_generics  {
			fn parse<#impl_with_generics>(state: &'parser Parser, hanging_gobble: Option<fso_tables::FSOParsingHangingGobble>) -> Result<(Self, Option<fso_tables::FSOParsingHangingGobble>), fso_tables::FSOParsingError> #where_clause_with_parser {
				const NONE_ARRAY_REPEAT_VALUE: Option<String> = None;
				let mut __comments = [NONE_ARRAY_REPEAT_VALUE; #field_count];
				let mut __version_strings = [NONE_ARRAY_REPEAT_VALUE; #field_count];
				let (mut __comment, mut __version_string, mut __already_parsed_comments) = if let Some(hanging_gobble) = hanging_gobble {
					(hanging_gobble.comments, hanging_gobble.version_string, true)
				} 
				else { (None, None, false) };
				#prefix_parser
				#parser
				#suffix_parser
				let __hanging_comments = if __already_parsed_comments { Some(fso_tables::FSOParsingHangingGobble {
					comments: __comment,
					version_string: __version_string
				}) } else { None };
				core::result::Result::Ok((#struct_name {
					#filler
					__comments,
					__version_strings
				}, __hanging_comments))
			}
			fn spew(&self, state: &mut impl fso_tables::FSOBuilder) {
				#prefix_spewer
				#spew
				#suffix_spewer
			}
		}
		impl #struct_name #ty_generics {
			pub fn new(#field_comma_list) -> Self{
				const NONE_ARRAY_REPEAT_VALUE: Option<String> = None;
				#struct_name {
					#filler
					__comments: [NONE_ARRAY_REPEAT_VALUE; #field_count],
					__version_strings: [NONE_ARRAY_REPEAT_VALUE; #field_count]
				}
			}	
		}
	}, 
	quote! { 
		impl #struct_name #ty_generics {
			pub fn parse<Parser>(parser: Parser) -> Result<Self, fso_tables::FSOParsingError> where Parser: for<'a> fso_tables::FSOParser<'a> { 
				let (parse, _) = fso_tables::FSOTable::parse(&parser, None)?;
				Ok(parse)
			}
			pub fn spew(&self) -> String {
				let mut parser = fso_tables::FSOTableBuilder::default();
				fso_tables::FSOTable::spew(self, &mut parser);
				fso_tables::FSOBuilder::spew(parser)
			}
		}
	}))
}