use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, ExprLit, ItemStruct, Lit, Meta, MetaNameValue, Type};
use syn::spanned::Spanned;
use crate::FSONaming::{ExistenceIsBool, Named, Unnamed};
use crate::TableField;
use crate::typehandler::{deduce_type, FSOValueType};
use crate::util::{fso_build_impl_generics, fso_build_where_clause};

pub fn fso_struct_build_parse(fields: &Vec<TableField>) -> Result<(TokenStream, TokenStream), Error> {
	let mut parse = quote! {
		let mut __already_parsed_comments = false;
		let (mut __comments, mut __version_string) = (None, None);
	};
	let mut fill = TokenStream::new();

	for (_field_num, field) in fields.iter().enumerate() {
		let name = &field.rust_token;

		let parse_comments = quote!{
			if !__already_parsed_comments {
				(__comments, __version_string) = state.consume_whitespace(false);
			}	
		};

		let (value_type, make_type) = deduce_type(&field.rust_type)?;
		let parse_value = match &field.fso_name {
			Named { fso_name } => {
				match value_type {
					FSOValueType::Option { .. } => {
						quote!{
							let #name = if let Ok(_) = state.consume_string(#fso_name) {
								__already_parsed_comments = false;
								//TODO process __comments, __version_string
								Some(#make_type?) //Named Optionals must be parseable
							}
							else {
								None()
							}
						}
					}
					_ => {
						quote!{
							__already_parsed_comments = false;
							state.consume_string(#fso_name)?;
							let #name = #make_type?;
						}
					}
				}
			}
			Unnamed => {
				match value_type {
					FSOValueType::Option { .. } => {
						quote!{
							let #name = if let Ok(data) = #make_type { //Unnamed Optionals can fail during parsing itself, that's assumed to be "non-existant"
								__already_parsed_comments = false;
								//TODO process __comments, __version_string
								Some(data)
							}
							else {
								None
							}
						}
					}
					_ => {
						quote!{
							__already_parsed_comments = false;
							let #name = #make_type?;
						}
					}
				}
			}
			ExistenceIsBool { fso_name } => {
				match value_type {
					FSOValueType::Direct { ty: Type::Path( path ) } if path.path.is_ident("bool") => {
						quote!{
							__already_parsed_comments = false;
							let #name = state.consume_string(#fso_name).is_ok();
						}
					}
					_ => {
						return Err(Error::new(field.rust_span, "Only variables of type bool can be existence-bool'd!"));
					}
				}
			}
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
	}

	Ok((parse, fill))
}

pub fn fso_table_struct(item_struct: &mut ItemStruct, instancing_req: Vec<TokenStream>, lifetime_req: Vec<TokenStream>, table_prefix: Option<String>, table_suffix: Option<String>) -> Result<TokenStream, Error> {
	let mut table_fields: Vec<TableField> = Vec::new();
	let struct_name = &item_struct.ident;
	let (_, ty_generics, where_clause) = item_struct.generics.split_for_impl();

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
			field.attrs.retain(|a| !(a.path().is_ident("fso_name") || a.path().is_ident("skip") || a.path().is_ident("unnamed") || a.path().is_ident("existence")));

			if skip.is_some() {
				continue;
			}

			if let Some(ident) = field.ident.as_ref() {
				let rust_token = ident.to_string();
				let fso_name;

				if unnamed.is_none() {
					let actual_name = forced_table_name.unwrap_or(Ok("$".to_string() + &rust_token[..1].to_string().to_uppercase() + &rust_token[1..] + ":"))?;
					if existence_is_bool.is_none() {
						fso_name = Named { fso_name: actual_name };
					}
					else {
						fso_name = ExistenceIsBool { fso_name: actual_name };
					}
				}
				else {
					fso_name = Unnamed;
				}

				table_fields.push(TableField { fso_name, rust_token: ident.clone(), rust_type, rust_span: field.span() });
			}
		}
		/*fields.named.push(
			syn::Field::parse_named
				.parse2(quote! { __unknown_fso_fields: Vec<String> })?,
		);*/
	}
	else {
		return Err(Error::new(item_struct.fields.span(), "A struct annotated with fso_table must have named fields!"));
	}

	let impl_with_generics = fso_build_impl_generics(&lifetime_req, &item_struct.generics);

	let where_clause_with_parser = fso_build_where_clause(&instancing_req, &where_clause);

	let (parser, filler) = fso_struct_build_parse(&table_fields)?;

	let prefix_parser = if let Some(prefix) = table_prefix{
		quote! {
			let (_, _) = state.consume_whitespace(false);
			state.consume_string(#prefix)?;
		}
	}
	else {
		quote!{}
	};
	let suffix_parser = if let Some(suffix) = table_suffix{
		quote! {
			let (_, _) = state.consume_whitespace(false);
			state.consume_string(#suffix)?;
		}
	}
	else {
		quote!{}
	};

	Ok(quote! {
		impl <#impl_with_generics> fso_tables::FSOTable<'parser, Parser> for #struct_name #ty_generics #where_clause_with_parser {
			fn parse(state: &'parser Parser) -> Result<#struct_name #ty_generics, fso_tables::FSOParsingError> {
				#prefix_parser
				#parser
				#suffix_parser
				core::result::Result::Ok(#struct_name {
					#filler
				})
			}
			fn dump(&self) { }
		}
	})
}