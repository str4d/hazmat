mod traits {
    /// This is a low-level trait that we want downstream users to implement but not use.
    /// We protect it by putting on a [`hazmat::suit`].
    #[hazmat::suit]
    pub trait AddOnce {
        fn add_once(self, other: &Self) -> Self;
    }

    /// This is a high-level trait that we want downstream users to use.
    pub trait AddTwice {
        fn add_twice(self, other: &Self) -> Self;
    }

    // We provide the high-level implementation in terms of the low-level implementation,
    // allowing downstream users to customise the internals without exposing the internals
    // in the public API.
    //
    // More precisely, the low-level implementation _is_ part of the public API, but it
    // cannot be used publicly because the capability type can only be constructed by the
    // trait author.
    impl<T: AddOnce> AddTwice for T {
        fn add_twice(self, other: &Self) -> Self {
            self.add_once(other, AddOnceCap(()))
                .add_once(other, AddOnceCap(()))
        }
    }
}

#[derive(Debug, PartialEq)]
struct Num(u32);

#[hazmat::suit]
impl traits::AddOnce for Num {
    fn add_once(self, other: &Self) -> Self {
        Self(self.0 + other.0)
    }
}

#[test]
fn test_high_level() {
    use crate::traits::AddTwice;

    let a = Num(7);
    let b = Num(15);
    let c = Num(37);
    assert_eq!(a.add_twice(&b), c);
}
