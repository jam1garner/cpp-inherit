# cpp-inherit
A macro for inheriting Rust structures from C++ classes. Nothing of value lies here.

### Example

```rust
use cpp_inherit::*;

#[inherit_from(BaseType)]
#[derive(Debug)]
struct RustType {}

#[inherit_from_impl(BaseType, "test.hpp")]
impl RustType {
    fn new() -> Self {
        Self {
            _base: BaseType { vtable_: RustType::VTABLE_ as _, value: 3 }
        }
    }

    #[overridden] fn x(&self) -> i32 {
        99
    }
}

// Now you can pass RustType as a BaseType, access any BaseType fields, call any BaseType methods (virtual or not), from either C++ or Rust
```

[Rest of example usage here](https://github.com/jam1garner/cpp-inherit-test)
