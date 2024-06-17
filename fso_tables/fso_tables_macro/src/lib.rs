use proc_macro::{TokenStream};
use proc_macro2::{Ident, Span};
use syn::{parse_macro_input, Item, Meta, Expr, Lit, ExprLit, MetaNameValue, ItemStruct, Path, Type, GenericArgument, TypePath, Token, PathSegment, parenthesized, LifetimeParam, ItemEnum, Generics, WhereClause, LitStr};
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::PathArguments::AngleBracketed;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{PathSep};
use crate::FSONaming::{ExistenceIsBool, Named, Unnamed};

enum FSONaming {
	Named { fso_name: String },
	Unnamed,
	ExistenceIsBool { fso_name: String }
}

struct TableField {
	fso_name: FSONaming,
	rust_token: Ident,
	rust_type: Type,
	rust_span: Span
}

fn fso_table_build_parse(fields: &Vec<TableField>) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
	let mut parse = quote! {
		let mut __already_parsed_comments = false;
		let (mut __comments, mut __version_string) = (None, None);
	};
	let mut fill = proc_macro2::TokenStream::new();
	
	for (_field_num, field) in fields.iter().enumerate() {
		let name = &field.rust_token;
		
		let new_parse = match &field.rust_type {
			Type::Path( TypePath { path: Path { segments, .. }, ..} ) if segments.last().map_or(false, |outer_type| outer_type.ident == "Option") => {
				let angle_brackets = 
				if let AngleBracketed(inner_types) = &segments.last().unwrap().arguments {
					inner_types.args.first().unwrap()
				}
				else {
					panic!("Unparametrized Option found!");
				};
				if let GenericArgument::Type( inner_type ) = &angle_brackets {
					let fso_name = match &field.fso_name {
						Named { fso_name } => { fso_name }
						_ => { 
							return (quote_spanned! {
								field.rust_span =>
								compile_error!("Options cannot be marked unnamed or existence-bool'd!");
							}, quote!());
						}
					};
					
					quote!(
						if !__already_parsed_comments {
							(__comments, __version_string) = state.consume_whitespace(false);
							__already_parsed_comments = true;
						}
						let #name
						if let Ok(_) = state.consume_string(#fso_name) {
							#name = Some(<#inner_type as fso_tables::FSOTable<Parser>>::parse(state)?);
							__already_parsed_comments = false;
							//TODO process __comments, __version_string
						}
						else {
							#name = None();
						}
					)
				}
				else {
					panic!("Unparametrized Option found!");
				}
			}
			Type::Path( TypePath { path: Path { segments, .. }, ..} ) if segments.last().map_or(false, |outer_type| outer_type.ident == "Vec") => {
				let angle_brackets =
					if let AngleBracketed(inner_types) = &segments.last().unwrap().arguments {
						inner_types.args.first().unwrap()
					}
					else {
						panic!("Unparametrized Vec found!");
					};
				if let GenericArgument::Type( Type::Path( TypePath { path , .. } ) ) = &angle_brackets {
					let inner_type = &path.segments.last().unwrap().ident;
					let fso_name_parse = match &field.fso_name {
						Named { fso_name } => { quote!(state.consume_string(#fso_name)?;) }
						Unnamed => { quote!() }
						ExistenceIsBool { .. } => {
							return (quote_spanned! {
								field.rust_span =>
								compile_error!("Vectors cannot be existence-bool'd!");
							}, quote!());
						}
					};
					
					quote!(
						if !__already_parsed_comments {
							(__comments, __version_string) = state.consume_whitespace(false);
						}
						#fso_name_parse
						__already_parsed_comments = false;
						let mut #name = Vec::default();
						//TODO process __comments, __version_string
						while let Ok(__new_element_for_vec) = <#inner_type as fso_tables::FSOTable<Parser>>::parse(state) {
							#name.push(__new_element_for_vec);
						}
					)
				}
				else {
					panic!("Unparametrized Vec found!");
				}
			}
			Type::Path( TypePath { path, ..} ) => {
				let fso_name_parse = match &field.fso_name {
					Named { fso_name } => { quote! {
						state.consume_string(#fso_name)?;
						let #name = <#path as fso_tables::FSOTable<Parser>>::parse(state)?;
					} }
					Unnamed => { quote!( let #name = <#path as fso_tables::FSOTable<Parser>>::parse(state)?; ) }
					ExistenceIsBool { fso_name } => {
						match &field.rust_type {
							Type::Path( TypePath { path: Path { segments, .. }, ..} ) if segments.last().map_or(false, |outer_type| outer_type.ident == "bool") => {
								quote! {
									let #name = if let Ok(_) = state.consume_string(#fso_name) {
										true
									}
									else {
										false
									}
								}
							}
							_ =>  {
								return (quote_spanned! {
									field.rust_span =>
									compile_error!("Non-bools cannot be existence-bool'd!");
								}, quote!());
							}
						}
					}
				};
				
				quote!(
					if !__already_parsed_comments {
						(__comments, __version_string) = state.consume_whitespace(false);
					}
					__already_parsed_comments = false;
					#fso_name_parse
					//TODO process __comments, __version_string
				)
			}
			_ => {
				quote_spanned! {
					field.rust_span =>
					compile_error!("Cannot process non-path types for FSO table parsing!");
				}
			}
		};
		
		parse = quote!(
			#parse
			#new_parse
		);
		
		fill = quote!(
			#fill
			#name,
		);
	}

	(parse, fill)
}

fn fso_enum_build_variant_parse(ident: &Ident, field_type: &Type) -> proc_macro2::TokenStream {
	match field_type {
		Type::Path( TypePath { path: Path { segments, .. }, ..} ) if segments.last().map_or(false, |outer_type| outer_type.ident == "Option") => {
			let angle_brackets =
				if let AngleBracketed(inner_types) = &segments.last().unwrap().arguments {
					inner_types.args.first().unwrap()
				} else {
					panic!("Unparametrized Option found!");
				};
			if let GenericArgument::Type(inner_type) = &angle_brackets {
				quote! {
					#ident: { 
						if let Ok(data) = <#inner_type as fso_tables::FSOTable<Parser>>::parse(state) {
							Some(data)
						}
						else { None }
					}
				}
			} else {
				panic!("Unparametrized Option found!");
			}
		}
		_ => { 
			quote! {
				#ident: { <#field_type as fso_tables::FSOTable<Parser>>::parse(state)? }
			}
		}
	}
}

fn fso_build_impl_generics(lifetime_req: &Vec<proc_macro2::TokenStream>, generics: &Generics) -> proc_macro2::TokenStream {
	let mut impl_with_generics = proc_macro2::TokenStream::new();
	for lifetime in lifetime_req.iter() {
		impl_with_generics = quote! {#impl_with_generics #lifetime, };
	}

	if !generics.params.is_empty() && lifetime_req.is_empty() {
		let inner_generics = generics.params.to_token_stream();
		impl_with_generics = quote! {#impl_with_generics #inner_generics, };
	}

	quote!{#impl_with_generics 'parser, Parser}
}

fn fso_build_where_clause(instancing_req: &Vec<proc_macro2::TokenStream>, where_clause: &Option<&WhereClause>) -> proc_macro2::TokenStream {
	let mut where_clause_with_parser = if let Some(where_clause) = where_clause {
		let inner_where = where_clause.to_token_stream();
		quote! {#inner_where, }
	}
	else {
		quote! {where }
	};
	
	for instancing_type in instancing_req.iter() {
		where_clause_with_parser = quote! {#where_clause_with_parser Parser: #instancing_type, };
	}

	where_clause_with_parser
}

fn fso_table_struct(item_struct: &mut ItemStruct, instancing_req: Vec<proc_macro2::TokenStream>, lifetime_req: Vec<proc_macro2::TokenStream>, table_prefix: Option<String>, table_suffix: Option<String>) -> proc_macro2::TokenStream {
	let mut table_fields: Vec<TableField> = Vec::new();
	let struct_name = &item_struct.ident;
	let (_, ty_generics, where_clause) = item_struct.generics.split_for_impl();
	
	if let syn::Fields::Named(ref mut fields) = item_struct.fields {
		for field in fields.named.iter_mut() {
			let rust_type = field.ty.clone();
			let forced_table_name = field.attrs.iter().find_map(|a| match &a.meta {
				Meta::NameValue( MetaNameValue { value: Expr::Lit( ExprLit{ lit: Lit::Str(new_name), ..}), path, .. })
				if path.is_ident("fso_name") => { Some(new_name.value()) },
				Meta::NameValue(MetaNameValue { path, .. }) if path.is_ident("fso_name") => {
					//TODO error here
					None
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
			let existance_is_bool = field.attrs.iter().find_map(|a| match &a.meta {
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
					let actual_name = forced_table_name.unwrap_or("$".to_string() + &rust_token[..1].to_string().to_uppercase() + &rust_token[1..] + ":");
					if existance_is_bool.is_none() {
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
				.parse2(quote! { __unknown_fso_fields: Vec<String> })
				.unwrap(),
		);*/
	}
	else {
		panic!("Could not add fields to table struct!");
	}
	
	let impl_with_generics = fso_build_impl_generics(&lifetime_req, &item_struct.generics);
	
	let where_clause_with_parser = fso_build_where_clause(&instancing_req, &where_clause);
	
	let (parser, filler) = fso_table_build_parse(&table_fields);
	
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
	
	quote! {
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
	}
}

fn fso_table_enum(item_enum: &mut ItemEnum, instancing_req: Vec<proc_macro2::TokenStream>, lifetime_req: Vec<proc_macro2::TokenStream>, prefix: String, suffix: String) -> proc_macro2::TokenStream {
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
				if option_nr == num_variants - 1 && option.fields.len() == 1 {
					Some(Ok(()))
				}
				else {
					Some(Err(quote_spanned! {
						option.span() => compile_error!("Only the last variant of an enum can be used as a default case and it must have exactly one String field.")
					}))
				}
			},
			_ => { None }
		});
		option.attrs.retain(|a| !a.path().is_ident("use_as_default_string"));

		let mut field_parsers = quote!();
		for field in &option.fields{
			if let Some(ident) = &field.ident {
				if let Some(target) = &default_enum_case_store_in{
					if let Err(err) = target {
						return err.clone();
					};
					
					field_parsers = quote! {
						#field_parsers
						#ident: state.read_until_whitespace(),
					};
				}
				else {
					let field_parser = fso_enum_build_variant_parse(ident, &field.ty);
					field_parsers = quote! {
						#field_parsers
						#field_parser,
					};
				}
			}
			else {
				return quote_spanned! {
					field.span() => compile_error!("FSO table enums cannot have unnamed fields.")
				};
			}
		}
		
		let rust_name = &option.ident;

		if let Some(_) = &default_enum_case_store_in {
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
	
	quote! {
		impl <#impl_with_generics> fso_tables::FSOTable<'parser, Parser> for #struct_name #ty_generics #where_clause_with_parser {
			fn parse(state: &'parser Parser) -> Result<#struct_name #ty_generics, fso_tables::FSOParsingError> {
				#parser
				#fail_return
			}
			fn dump(&self) { }
		}
	}
}

#[proc_macro_attribute]
pub fn fso_table(args: TokenStream, input: TokenStream) -> TokenStream  {
	let mut pre_item_out = proc_macro2::TokenStream::new();
	let mut item = parse_macro_input!(input as Item);
	let mut post_item_out = proc_macro2::TokenStream::new();

	let mut required_parser_traits: Vec<proc_macro2::TokenStream> = vec![quote!(fso_tables::FSOParser<'parser>)];
	let mut required_lifetimes: Vec<proc_macro2::TokenStream> = vec![];
	
	let mut enum_prefix = "".to_string();
	let mut enum_suffix = "".to_string();

	let mut table_prefix :Option<String> = None;
	let mut table_suffix :Option<String> = None;
	
	struct ReqTraitParser {
		data: Punctuated::<PathSegment, PathSep>
	}
	impl Parse for ReqTraitParser{
		fn parse(tokens: ParseStream) -> syn::Result<ReqTraitParser> {
			let parser = Punctuated::<PathSegment, PathSep>::parse_separated_nonempty;
			let result = ReqTraitParser{ data: parser(tokens)? };
			Ok(result)
		}
	}
	
	let args_parser = syn::meta::parser(|meta| {
		if meta.path.is_ident("required_parser_trait") {
			let content;
			parenthesized!(content in meta.input);
			
			for req_trait in &content.parse_terminated(ReqTraitParser::parse, Token![,])? {
				let trait_path = Path { leading_colon: None, segments: req_trait.data.clone() };
				required_parser_traits.push(quote!(#trait_path));
			};
			Ok(())
		}
		else if meta.path.is_ident("required_lifetime") {
			let content;
			parenthesized!(content in meta.input);

			for lifetime in &content.parse_terminated(LifetimeParam::parse, Token![,])? {
				required_lifetimes.push(quote!(#lifetime));
			};
			Ok(())
		}
		else if meta.path.is_ident("prefix") {
			enum_prefix = meta.value()?.parse::<LitStr>()?.value();
			Ok(())
		}
		else if meta.path.is_ident("suffix") {
			enum_suffix = meta.value()?.parse::<LitStr>()?.value();
			Ok(())
		}
		else if meta.path.is_ident("table_start") {
			table_prefix = Some(meta.value()?.parse::<LitStr>()?.value());
			Ok(())
		}
		else if meta.path.is_ident("table_end") {
			table_suffix = Some(meta.value()?.parse::<LitStr>()?.value());
			Ok(())
		}
		else {
			Err(meta.error("Unsupported FSO table property"))
		}
	});
	parse_macro_input!(args with args_parser);

	match &mut item {
		Item::Struct(item_struct) => {
			post_item_out = fso_table_struct(item_struct, required_parser_traits, required_lifetimes, table_prefix, table_suffix);
		}
		Item::Enum(item_enum) => {
			post_item_out = fso_table_enum(item_enum, required_parser_traits, required_lifetimes, enum_prefix, enum_suffix);
		}
		_ => {
			pre_item_out = quote_spanned! {
                item.span() =>
                compile_error!("Can only annotate structs!");
            };
		}
	}

	return quote! {
        #pre_item_out
        #item
        #post_item_out
    }.into();
}