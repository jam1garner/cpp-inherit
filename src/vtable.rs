use std::collections::HashMap;
use std::env;
use std::io::prelude::*;

use quote::{format_ident, quote, ToTokens};
use std::process::{Command, Stdio};
use syn::{Ident, Path, Type};

mod dwarf;
use dwarf::VTableElement;

pub fn generate_vtable_const(methods: Vec<Path>, ty: &Type) -> impl ToTokens {
    let method_count = methods.len();
    quote!(
        impl #ty {
            // One constant to do a static borrow to ensure it's effectively a static
            const _VTABLE_BORROW_FDKSLASDASD: &'static [*const (); #method_count] = &[
                #(
                    #methods as *const (),
                )*
            ];

            // One constant to convert to a pointer to reduce casting
            //
            // TODO: is it possible to get the bindgen vtable type? if so then no casting would be
            // needed...
            const VTABLE_: *const [*const (); #method_count] = #ty::_VTABLE_BORROW_FDKSLASDASD as *const _;
        }
    )
}

pub fn get_vtable_info(header: &str, class: &str) -> HashMap<String, Vec<VTableElement>> {
    let header_path = env::current_dir().unwrap().join("src").join(header);
    let out_path = std::path::Path::new(&env::var("OUT_DIR").unwrap()).join(class);
    // Compile the header to an unstripped object file to 
    let mut gcc = Command::new("g++")
        .args(&[
            // I don't really know why some of these can't be removed but probably best to leave
            // these be
            "-femit-class-debug-always",
            "-fno-eliminate-unused-debug-types",
            "-fno-eliminate-unused-debug-symbols",
            "-g3",
            "-gdwarf-4",
            "-x",
            "c++",
            "-c",
        ])
        .arg("-o")
        .arg(&out_path)
        .arg(&header_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start g++");
    if !gcc.wait().unwrap().success() {
        let mut x = String::new();
        gcc.stderr.unwrap().read_to_string(&mut x).unwrap();
        panic!("g++ error:\n{}", x);
    }

    dwarf::get_vtables_from_file(&out_path)
}

pub fn get_binding_symbol(symbol: &str) -> Ident {
    format_ident!("__cpp_inherit_internal_{}", symbol)
}

pub fn generate_binding(symbol: &str) -> impl ToTokens {
    let ident = get_binding_symbol(symbol);

    quote!(
        extern "C" {
            #[link_name = #symbol]
            fn #ident();
        }
    )
}
