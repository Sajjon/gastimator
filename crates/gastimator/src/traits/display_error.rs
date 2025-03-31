/// A trait for unwrapping a `Result` with a `Display`-able error.
pub trait DisplayError<T, E: std::fmt::Display> {
    /// Unwrap the result, panicking with the error message using `Display`
    /// if it is an `Err`.
    fn unwrap_display(self) -> T;
}

impl<T, E: std::fmt::Display> DisplayError<T, E> for std::result::Result<T, E> {
    fn unwrap_display(self) -> T {
        self.unwrap_or_else(|e| panic!("❌ {} ❌", e))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_display_error() {
        #[derive(Debug, thiserror::Error)]
        pub enum MyErr {
            #[error("My string")]
            MyErrorVariant,
        }
        let result: Result<(), MyErr> = Err(MyErr::MyErrorVariant);
        // Capture the panic
        let panic_result = std::panic::catch_unwind(|| {
            result.unwrap_display(); // This should panic
        });

        // Assert that it panicked
        assert!(
            panic_result.is_err(),
            "Expected function to panic but it didn't"
        );

        // Extract panic message and assert its contents
        if let Err(err) = panic_result {
            let panic_msg = err
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| err.downcast_ref::<&str>().copied())
                .unwrap_or("<unknown panic message>");

            assert!(
                panic_msg.contains("My string"),
                "Expected panic message to contain 'My string', but got: {}",
                panic_msg
            );
        }
    }
}
