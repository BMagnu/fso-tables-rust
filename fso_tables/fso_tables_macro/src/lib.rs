mod typehandler;
mod r#struct;
mod r#enum;
mod util;

use proc_macro2::TokenStream;
use syn::{parse_macro_input, Item, Path, Token, PathSegment, parenthesized, LifetimeParam, LitStr, Error};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::PathSep;
use crate::r#struct::*;
use crate::r#enum::*;

#[proc_macro_attribute]
pub fn fso_table(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream  {
	let mut item = parse_macro_input!(input as Item);

	let mut required_parser_traits: Vec<TokenStream> = vec![quote!(fso_tables::FSOParser<'parser>)];
	let mut required_lifetimes: Vec<TokenStream> = vec![];
	
	let mut prefix :Option<String> = None;
	let mut suffix :Option<String> = None;

	let mut table_prefix :Option<String> = None;
	let mut table_suffix :Option<String> = None;
	
	let mut field_spacing :Option<String> = None;

	let mut flagset_naming = false;
	let mut inline = false;
	let mut toplevel = false;
	
	struct ReqTraitParser {
		data: Punctuated<PathSegment, PathSep>
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
			prefix = Some(meta.value()?.parse::<LitStr>()?.value());
			Ok(())
		}
		else if meta.path.is_ident("suffix") {
			suffix = Some(meta.value()?.parse::<LitStr>()?.value());
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
		else if meta.path.is_ident("flagset") {
			flagset_naming = true;
			Ok(())
		}
		else if meta.path.is_ident("inline") {
			inline = true;
			Ok(())
		}
		else if meta.path.is_ident("toplevel") {
			toplevel = true;
			Ok(())
		}
		else if meta.path.is_ident("enum_field_spacing") {
			field_spacing = Some(meta.value()?.parse::<LitStr>()?.value());
			Ok(())
		}
		else {
			Err(meta.error("Unsupported FSO table property"))
		}
	});
	parse_macro_input!(args with args_parser);

	let result = match &mut item {
		Item::Struct(item_struct) => {
			fso_table_struct(item_struct, required_parser_traits, required_lifetimes, table_prefix, table_suffix, prefix, suffix, inline)
		}
		Item::Enum(item_enum) => {
			fso_table_enum(item_enum, required_parser_traits, required_lifetimes, prefix.unwrap_or("".to_string()), suffix.unwrap_or("".to_string()), flagset_naming, field_spacing.unwrap_or(" ".to_string()))
		}
		_ => {
			Err(Error::new(item.span(), "Can only annotate structs and enums!"))
		}
	};
	
	let (post_item_out, top_level_impl) = match result {
		Ok(stream) => { stream }
		Err(error) => { (error.to_compile_error(), quote!()) }
	};
	let top_level_impl = if toplevel { top_level_impl } else { quote!() };
	
	return quote! {
        #item
        #post_item_out
		#top_level_impl
    }.into();
}