use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::{Error, Expr, ExprLit, Fields, ItemEnum, Lit, Meta, MetaNameValue};
use syn::spanned::Spanned;
use crate::typehandler::{deduce_type, FSONaming, FSOValueType};
use crate::util::{fso_build_impl_generics, fso_build_where_clause};

pub(crate) fn fso_enum_build_parse(fields: &Fields, default_enum_case_store_in: bool, field_spacing: &String) -> Result<(TokenStream, TokenStream), Error> {
	let mut field_parsers = quote!();
	let mut field_spewers = quote!();

	let mut field_number = 0u32;
	
	for field in fields{
		let append_space = if field_number == 0 { quote!(state.append(#field_spacing);) } else { quote!(state.append(", ");) };
		if let Some(ident) = &field.ident {
			if default_enum_case_store_in {
				field_parsers = quote! {
					#field_parsers
					#ident: state.read_until_whitespace(),
				};
				field_spewers = quote! {
					#field_spewers
					state.append(#ident.as_str());
				};
			}
			else {
				let (value_type, field_parser, field_spewer) = deduce_type(&FSONaming::Unnamed, &field.ty, &format_ident!("__field"))?;
				match value_type {
					FSOValueType::Option { .. } => {
						field_parsers = quote! {
							#field_parsers
							#ident: if let Ok(data) = #field_parser { Some(data) } else { None },
						};
						field_spewers = quote! {
							#field_spewers
							if let Some(__field) = #ident {
								#append_space
								#field_spewer
							}
						};
					}
					_ => {
						field_parsers = quote! {
							#field_parsers
							#ident: #field_parser?,
						};
						field_spewers = quote! {
							#field_spewers
							{
								let __field = #ident;
								#append_space
								#field_spewer
							}
						};
					}
				}
			}
		}
		else {
			return Err(Error::new(field.span(), "FSO table enums cannot have unnamed fields."));
		}
		field_number += 1;
	}
	
	Ok((field_parsers, field_spewers))
}

pub fn fso_table_enum(item_enum: &mut ItemEnum, instancing_req: Vec<TokenStream>, lifetime_req: Vec<TokenStream>, prefix: String, suffix: String, flagset_naming: bool, field_spacing: String) -> Result<(TokenStream, TokenStream), Error> {
	let struct_name = &item_enum.ident;
	let (_, ty_generics, where_clause) = item_enum.generics.split_for_impl();

	let mut parser = quote!(let (_, _) = state.consume_whitespace(false););
	let mut spewer = quote!();
	let mut fail_message = "Expected one of ".to_string();
	let mut option_nr = 0;

	let mut has_early_out = false;
	let num_variants = item_enum.variants.len();

	for option in &mut item_enum.variants {
		let default_enum_case_store_in = option.attrs.iter().find_map(|a| match &a.meta {
			Meta::Path( path ) if path.is_ident("use_as_default_string") => {
				if option_nr != num_variants - 1{
					Some(Err(Error::new(option.span(), "Only the last variant of an enum can be used as a default case.")))
				} 
				else if option.fields.len() != 1 {
					Some(Err(Error::new(option.span(), "An enum default case must have exactly one field of the type String.")))
				}
				else {
					Some(Ok(true))
				}
			},
			_ => { None }
		});

		let forced_table_name = option.attrs.iter().find_map(|a| match &a.meta {
			Meta::NameValue( MetaNameValue { value: Expr::Lit( ExprLit{ lit: Lit::Str(new_name), ..}), .. })
			if a.meta.path().is_ident("fso_name") => { Some(Ok(new_name.value())) },
			_ if a.meta.path().is_ident("fso_name") => {
				return Some(Err(Error::new(a.span(), "Attribute fso_name must have a value!")));
			}
			_ => {
				None
			}
		});
		option.attrs.retain(|a| !(a.path().is_ident("use_as_default_string") || a.path().is_ident("fso_name")));

		thread_local! { static UPPERCASE_REGEX: Regex = Regex::new(r"([A-Z][a-z]*)").unwrap(); }

		let fso_name = if let Some(forced_name) = forced_table_name {
			forced_name?
		}
		else if flagset_naming {
			let mut out_name = "".to_string();
			UPPERCASE_REGEX.with(|regex| {
				for (number, name_part) in regex.find_iter(&option.ident.to_string()).enumerate() {
					out_name = format!("{}{}{}", out_name, if number == 0 { "" } else { " " }, name_part.as_str().to_lowercase());
				}
			});
			out_name
		}
		else {
			let mut out_name = "".to_string();
			UPPERCASE_REGEX.with(|regex| {
				for (number, name_part) in regex.find_iter(&option.ident.to_string()).enumerate() {
					out_name = format!("{}{}{}", out_name, if number == 0 { "" } else { " " }, name_part.as_str());
				}
			});
			out_name
		};

		let fso_name = format!("{}{}{}", prefix, fso_name, suffix);
		fail_message = format!("{}{}, ", fail_message, fso_name);

		let default_enum_case_store_in = default_enum_case_store_in.unwrap_or(Ok(false))?;
		let (field_parsers, field_spewers) = fso_enum_build_parse(&option.fields, default_enum_case_store_in, &field_spacing)?;

		let rust_name = &option.ident;

		if default_enum_case_store_in {
			has_early_out = true;
			parser = quote! {
				#parser
				return Ok( #struct_name::#rust_name {
					#field_parsers
				});
			};
		}
		else {
			parser = quote! {
				#parser
				if let Ok(_) = state.consume_string(#fso_name) {
					return Ok( #struct_name::#rust_name {
						#field_parsers
					});
				}
			};
		}

		let field_names = option.fields.iter().fold(quote!(), |input, next| {
			if let Some(next_name) = &next.ident {
				quote!(#input #next_name,)
			}
			else {
				input
			}
		});
		
		let prespew = if flagset_naming { quote!(state.append("\"");) } else { quote!() };

		if default_enum_case_store_in {
			spewer = quote! {
				#spewer
				#struct_name::#rust_name { #field_names } => {
					#prespew
					#field_spewers
					#prespew
					state.append(" ");
				}
			};
		}
		else {
			spewer = quote! {
				#spewer
				#struct_name::#rust_name { #field_names } => {
					#prespew
					state.append(#fso_name);
					#field_spewers
					#prespew
					state.append(" ");
				}
			};
		}

		option_nr += 1;
	}

	fail_message = format!("{}got {{}}.", fail_message);
	let fail_return = if has_early_out {
		quote!()
	}
	else{
		quote! {
			let current = state.get();
			let current_cut = &current[..std::cmp::min(20, current.len())];
			core::result::Result::Err(fso_tables::FSOParsingError { reason: format!(#fail_message, current_cut), line: state.line() })
		}
	};

	let impl_with_generics = fso_build_impl_generics(&lifetime_req, &item_enum.generics);

	let where_clause_with_parser = fso_build_where_clause(&instancing_req, &where_clause);

	Ok((quote! {
		impl <#impl_with_generics> fso_tables::FSOTable<'parser, Parser> for #struct_name #ty_generics #where_clause_with_parser {
			fn parse(state: &'parser Parser) -> Result<#struct_name #ty_generics, fso_tables::FSOParsingError> {
				#parser
				#fail_return
			}
			fn spew(&self, state: &mut impl fso_tables::FSOBuilder) {
				match self {
					#spewer
				}
			}
		}
	},
	quote! { 
		impl #struct_name #ty_generics {
			pub fn parse<Parser>(parser: Parser) -> Result<Self, fso_tables::FSOParsingError> where Parser: for<'parser> fso_tables::FSOParser<'parser> { 
				fso_tables::FSOTable::parse(&parser)
			}
		}
	}))
}