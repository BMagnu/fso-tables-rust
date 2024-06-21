use std::cmp::PartialEq;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{GenericArgument, Path, Type, TypePath, Error, TypeTuple, Index};
use syn::PathArguments::AngleBracketed;
use syn::spanned::Spanned;

#[derive(PartialEq)]
pub(crate) enum FSONaming {
	Named { fso_name: String, multiline: bool },
	Unnamed,
	ExistenceIsBool { fso_name: String },
	Skipped
}

#[allow(dead_code)]
pub(crate) enum FSOValueType<'a> {
	Direct {ty: &'a Type},
	Generic {ty: &'a Type},
	Vector {inner: &'a Type},
	Option {inner: &'a Type},
	Container {inner: &'a Type, container: &'a Ident},
	Tuple {inner: Vec<FSOValueType<'a>>}
}

pub(crate) fn deduce_type<'a>(name: &FSONaming, ty: &'a Type, to_spew_name: &Ident) -> Result<(FSOValueType<'a>, TokenStream, TokenStream), Error>{
	match ty {
		Type::Path( TypePath { path: Path { segments, .. }, ..} ) => {
			assert!(!segments.is_empty());
			let typename = segments.last().unwrap();
			if let AngleBracketed(inner_types) = &typename.arguments {
				if let GenericArgument::Type( inner ) = inner_types.args.first().unwrap() {
					match typename.ident.to_string().as_str() {
						"Vec" => {
							let multiline = if let FSONaming::Named { multiline, ..} = name { *multiline } else if *name == FSONaming::Unnamed { true } else { false };
							
							let (inner_type, make_containing, spew_containing) = deduce_type(name, inner, &format_ident!("__to_spew"))?;
							if let FSOValueType::Option { .. } = inner_type {
								return Err(Error::new(inner.span(), "FSO Tables cannot contain a Vector of Options. Consider adding a subtable with optional unnamed elements."));
							}

							let make_value = quote!{
								{
									let mut __vec_to_fill = Vec::new();
									state.consume_whitespace_inline(&['(']);
									while let Ok(__new_element_for_vec) = #make_containing {
										__vec_to_fill.push(__new_element_for_vec);
									}
									state.consume_whitespace_inline(&[')']);
									Ok(__vec_to_fill)
								}
							};

							let push = if multiline {
								quote! {
									state.get_state().list_state.push(fso_tables::FSOBuilderListState::MutlilineList);
								}
							}
							else {
								quote! {
									state.append("(");
									state.get_state().list_state.push(fso_tables::FSOBuilderListState::InlineList);
								}
							};
							let pop = if multiline {
								quote! {
									state.get_state().list_state.pop();
								}
							}
							else {
								quote! {
									state.get_state().list_state.pop();
									state.append(")");
								}
							};
							let newline = if multiline { quote!(state.append("\n");) } else { quote!() };
							
							let spew_value = quote!{
								{
									#push
									for __to_spew in #to_spew_name.iter() {
										#newline
										#spew_containing
									}
									#pop
								}
							};
							
							Ok((FSOValueType::Vector { inner }, make_value, spew_value))
						}
						"Option" => {
							let (inner_type, make_containing, spew_containing) = deduce_type(name, inner, to_spew_name)?;
							if let FSOValueType::Option { .. } | FSOValueType::Container { .. } = inner_type {
								return Err(Error::new(inner.span(), "FSO Tables cannot contain an Option of Options or Box-likes. Consider reversing the template order."));
							}

							let make_value = quote! (#make_containing);
							let spew_value = quote! (#spew_containing);

							Ok((FSOValueType::Option { inner }, make_value, spew_value))
						}
						"Box" | "Rc" | "Arc" | "Cell" | "RefCell" => {
							let (inner_type, make_containing, spew_containing) = deduce_type(name, inner, &format_ident!("__box_contained"))?;
							if let FSOValueType::Option { .. } = inner_type {
								return Err(Error::new(inner.span(), "FSO Tables cannot contain a Box-like of Options. Consider reversing the template order."));
							}
							
							let container = &typename.ident;
							let make_value = quote!(#make_containing.map(|containing| #container::new(containing)));
							let spew_value = quote!{
								{
									let __box_contained = &#to_spew_name.as_ref();
									#spew_containing
								}
							};
							
							Ok((FSOValueType::Container { inner, container }, make_value, spew_value))
						}
						_ => {
							let make_value = quote! (<#ty as fso_tables::FSOTable>::parse(state));
							let spew_value = quote! (<#ty as fso_tables::FSOTable>::spew(#to_spew_name, state););
							Ok((FSOValueType::Generic { ty }, make_value, spew_value))
						}
					}
				}
				else {
					Err(Error::new(ty.span(), format!("FSO Tables encountered type {} with non-type generic argument!", typename.ident)))
				}
			} else {
				let make_value = quote! (<#ty as fso_tables::FSOTable>::parse(state));
				let spew_value = quote! (<#ty as fso_tables::FSOTable>::spew(#to_spew_name, state););
				Ok((FSOValueType::Direct { ty }, make_value, spew_value))
			}
		}
		Type::Tuple( TypeTuple { elems, .. } ) => {
			let mut types: Vec<FSOValueType> = Vec::new();
			let mut parser = quote!();
			let mut spewer = quote!();
			let mut count = 0;

			for inner in elems {
				let (inner_type, make_containing, spew_containing) = deduce_type(name, inner, &format_ident!("__current_enum"))?;
				if let FSOValueType::Option { .. } = inner_type {
					return Err(Error::new(inner.span(), "FSO Tables cannot yet contain Options."));
				}
				types.push(inner_type);
				parser = quote!{
					#parser
					#make_containing?,
				};

				let spew_comma = if count == 0 {
					quote!()
				}
				else {
					quote!(state.append(", ");)
				};

				let count_name = Index::from(count);

				spewer = quote!{
					#spewer
					#spew_comma
					{
						let __current_enum = &#to_spew_name.#count_name;
						#spew_containing
					}
				};

				count += 1;
			}

			let parse_value = quote!{ (|| {
				state.consume_whitespace_inline(&['(']);
				let __tuple_result = Ok((#parser));
				state.consume_whitespace_inline(&[')']);
				__tuple_result
			})() };

			let spew_value = quote! {
				{
					state.append("(");
					#spewer
					state.append(")");
				}
			};

			Ok((FSOValueType::Tuple { inner: types }, parse_value, spew_value))
		}
		_ => {
			Err(Error::new(ty.span(), "FSO Tables can only process path and tuple types"))
		}
	}
}