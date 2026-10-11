//! Collecting every error in a declaration, so that one compile reports them
//! all.

/// The errors found so far.
#[derive(Default)]
pub(crate) struct Errors(Option<syn::Error>);

impl Errors {
    pub(crate) fn push(&mut self, error: syn::Error) {
        match &mut self.0 {
            Some(errors) => errors.combine(error),
            None => self.0 = Some(error),
        }
    }

    /// The value of `result`, or `None` with its error kept.
    pub(crate) fn keep<T>(&mut self, result: syn::Result<T>) -> Option<T> {
        result.map_err(|error| self.push(error)).ok()
    }

    pub(crate) fn finish(self) -> syn::Result<()> {
        self.0.map_or(Ok(()), Err)
    }
}
