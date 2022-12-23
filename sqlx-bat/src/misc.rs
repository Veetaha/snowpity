use easy_ext::ext;

#[ext(ErrorExt)]
pub impl sqlx::Error {
    fn is_constraint_violation(&self, constraint: &str) -> bool {
        self.as_database_error()
            .map(|err| err.constraint() == Some(constraint))
            .unwrap_or(false)
    }
}
