use cpp_inherit::*;

include!("test.rs");

#[inherit_from(base)]
#[derive(Debug)]
struct Test {}

#[inherit_from_impl(base, "test.hpp")]
impl Test {
    fn new() -> Self {
        Self {
            _base: base { vtable_: Test::VTABLE_ as _, value: 3 }
        }
    }

    #[overridden] fn x(&self) -> i32 {
        99
    }
}

fn main() {
    let test = Test::new();
    dbg!(test.value);
    dbg!(test.x());
}
