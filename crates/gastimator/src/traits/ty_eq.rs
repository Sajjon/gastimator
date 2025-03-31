/// Workaround for #20041
/// https://github.com/rust-lang/rust/issues/20041
pub trait TyEq<T> {
    /// Casts `self: Self` to `T`, since `Self` and `T` are the same type.
    fn cast(self) -> T;
}

impl<T> TyEq<T> for T {
    fn cast(self) -> T {
        self
    }
}
