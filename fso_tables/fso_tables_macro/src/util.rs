use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Generics, WhereClause};

pub fn fso_build_impl_generics(lifetime_req: &Vec<TokenStream>, generics: &Generics) -> TokenStream {
	let mut impl_with_generics = TokenStream::new();
	for lifetime in lifetime_req.iter() {
		impl_with_generics = quote! {#impl_with_generics #lifetime, };
	}

	if !generics.params.is_empty() && lifetime_req.is_empty() {
		let inner_generics = generics.params.to_token_stream();
		impl_with_generics = quote! {#impl_with_generics #inner_generics, };
	}

	quote! {#impl_with_generics 'parser, Parser}
}

pub fn fso_build_where_clause(instancing_req: &Vec<TokenStream>, where_clause: &Option<&WhereClause>) -> TokenStream {
	let mut where_clause_with_parser = if let Some(where_clause) = where_clause {
		let inner_where = where_clause.to_token_stream();
		quote! {#inner_where, }
	}
	else {
		quote! {where }
	};

	for instancing_type in instancing_req.iter() {
		where_clause_with_parser = quote! {#where_clause_with_parser Parser: #instancing_type };
	}

	where_clause_with_parser
}