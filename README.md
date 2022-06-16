# `hazmat` Rust library

A collection of helpers for working with hazardous materials in Rust crates.

## `#[hazmat::suit]`

Ever needed to expose an internal trait for downstream users to implement, that
shouldn't be usable outside of your crate? Add the `#[hazmat::suit]` attribute
to the trait and it becomes "implement-only":

```rust
#[hazmat::suit]
pub trait LowLevel {
    fn low_level(self) -> bool;
}

struct DownstreamType;

#[hazmat::suit]
impl LowLevel for DownstreamType {
    fn low_level(self, other: Self) -> bool {
        true
    }
}

fn use_low_level() {
    let a = DownstreamType;
    let b = DownstreamType;

    // This won't compile outside of the crate in which the trait is defined.
    assert!(a.low_level(b, LowLevelCap));
}
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
