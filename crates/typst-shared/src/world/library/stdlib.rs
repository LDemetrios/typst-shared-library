// Added by LDemetrios

use typst::LibraryExt;
use typst::utils::SmallBitSet;
use typst_library::foundations::{Binding, Dict, Module, Scope, Value};
use typst_library::{Features, Library};

pub fn stdlib(features: SmallBitSet) -> Library {
    Library::builder()
        .with_inputs(Dict::new())
        .with_features(Features(features))
        .build()
}

pub fn replace_inputs(lib: &mut Library, inputs: Dict) {
    // inputs, by default, are included in `lib` twice: in `global` and in `std`

    change_sys(&mut lib.global, &inputs);

    if let Value::Module(std) = lib.std.write().unwrap() {
        change_sys(std, &inputs);
    }
}

fn change_sys(lib: &mut Module, inputs: &Dict) {
    if let Some(sys) = lib.scope_mut().get_mut("sys") {
        if let Value::Module(sys) = sys.write().unwrap() {
            sys.scope_mut().define_unchecked("inputs", inputs.clone());
        }
    }
}
