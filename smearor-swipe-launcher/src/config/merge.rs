#[allow(unused)]
pub trait MergeWithArguments<T> {
    /// Merges the configuration file of type T with the given command line arguments.
    fn merge_with_arguments(self, args: &T) -> Self;
}
