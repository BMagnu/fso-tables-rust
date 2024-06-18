use proc_macro2::{Ident, TokenStream};
use quote::{quote};
use syn::{GenericArgument, Path, Type, TypePath, Error};
use syn::PathArguments::AngleBracketed;
use syn::spanned::Spanned;

pub enum FSOValueType<'a> {
	Direct {ty: &'a Type},
	Generic {ty: &'a Type},
	Vector {inner: &'a Type},
	Option {inner: &'a Type},
	Container {inner: &'a Type, container: &'a Ident}
}

pub fn deduce_type(ty: &Type) -> Result<(FSOValueType, TokenStream), Error>{
	match ty {
		Type::Path( TypePath { path: Path { segments, .. }, ..} ) => {
			assert!(!segments.is_empty());
			let typename = segments.last().unwrap();
			if let AngleBracketed(inner_types) = &typename.arguments {
				if let GenericArgument::Type( inner ) = inner_types.args.first().unwrap() {
					match typename.ident.to_string().as_str() {
						"Vec" => {
							let (inner_type, make_containing) = deduce_type(inner)?;
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
							
							Ok((FSOValueType::Vector { inner }, make_value))
						}
						"Option" => {
							let (inner_type, make_containing) = deduce_type(inner)?;
							if let FSOValueType::Option { .. } | FSOValueType::Container { .. } = inner_type {
								return Err(Error::new(inner.span(), "FSO Tables cannot contain an Option of Options or Box-likes. Consider reversing the template order."));
							}

							let make_value = quote! (#make_containing);
							Ok((FSOValueType::Option { inner }, make_value))
						}
						"Box" | "Rc" | "Arc" | "Cell" | "RefCell" => {
							let (_, make_containing) = deduce_type(inner)?;
							
							let container = &typename.ident;
							let make_value = quote!(#make_containing.map(|containing| #container::new(containing)));
							
							Ok((FSOValueType::Container { inner, container }, make_value))
						}
						_ => {
							let make_value = quote! (<#ty as fso_tables::FSOTable<Parser>>::parse(state));
							Ok((FSOValueType::Generic { ty }, make_value))
						}
					}
				}
				else {
					Err(Error::new(ty.span(), format!("FSO Tables encountered type {} with non-type generic argument!", typename.ident)))
				}
			} else {
				let make_value = quote! (<#ty as fso_tables::FSOTable<Parser>>::parse(state));
				Ok((FSOValueType::Direct { ty }, make_value))
			}
		}
		_ => {
			Err(Error::new(ty.span(), "FSO Tables can only process path and tuple types"))
		}
	}
}