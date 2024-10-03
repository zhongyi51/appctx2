use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;


enum InjectInfo{
    AutoWire{
        obj_name:String
    },
    Value{
        env_name:String
    }
}

struct FieldInfo{
    name:String,
    ty:String,
    inject_info:InjectInfo
}

struct ExportInfo{
    export_as:Vec<String>,
    export_name:String
}

struct StructInfo{
    name:String,
    export_info:ExportInfo,
    fields:Vec<FieldInfo>
}



#[proc_macro_derive(AppObj, attributes(appobj))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    println!("res is {:#?}",parsed);

    TokenStream::from(quote! {})
}
