use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, ItemEnum, Meta};
use syn::spanned::Spanned;
use crate::typehandler::{deduce_type, FSOValueType};
use crate::util::{fso_build_impl_generics, fso_build_where_clause};

pub(crate) fn fso_enum_build_parse(fields: &Fields, default_enum_case_store_in: bool) -> Result<TokenStream, Error> {
	let mut field_parsers = quote!();
	for field in fields{
		if let Some(ident) = &field.ident {
			if default_enum_case_store_in {
				field_parsers = quote! {
					#field_parsers
					#ident: state.read_until_whitespace(),
				};
			}
			else {
				let (value_type, field_parser) = deduce_type(&field.ty)?;
				match value_type {
					FSOValueType::Option { .. }=> {
						field_parsers = quote! {
							#field_parsers
							#ident: if let Ok(data) = #field_parser { Some(data) } else { None },
						};
					}
					_ => {
						field_parsers = quote! {
							#field_parsers
							#ident: #field_parser?,
						};
					}
				}
			}
		}
		else {
			return Err(Error::new(field.span(), "FSO table enums cannot have unnamed fields."));
		}
	}
	
	Ok(field_parsers)
}

pub fn fso_table_enum(item_enum: &mut ItemEnum, instancing_req: Vec<TokenStream>, lifetime_req: Vec<TokenStream>, prefix: String, suffix: String) -> Result<TokenStream, Error> {
	let struct_name = &item_enum.ident;
	let (_, ty_generics, where_clause) = item_enum.generics.split_for_impl();

	let mut parser = quote!();
	let mut fail_message = "Expected one of ".to_string();
	let mut option_nr = 0;

	let mut has_early_out = false;
	let num_variants = item_enum.variants.len();

	for option in &mut item_enum.variants {
		let fso_name = format!("{}{}{}", prefix, option.ident, suffix);
		fail_message = format!("{}{}, ", fail_message, fso_name);

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
		option.attrs.retain(|a| !a.path().is_ident("use_as_default_string"));

		let default_enum_case_store_in = default_enum_case_store_in.unwrap_or(Ok(false))?;
		let field_parsers = fso_enum_build_parse(&option.fields, default_enum_case_store_in)?;

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

	Ok(quote! {
		impl <#impl_with_generics> fso_tables::FSOTable<'parser, Parser> for #struct_name #ty_generics #where_clause_with_parser {
			fn parse(state: &'parser Parser) -> Result<#struct_name #ty_generics, fso_tables::FSOParsingError> {
				#parser
				#fail_return
			}
			fn dump(&self) { }
		}
	})
}