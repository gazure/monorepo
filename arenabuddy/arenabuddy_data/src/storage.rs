use arenabuddy_core::replay::MatchReplay;

#[expect(async_fn_in_trait)]
pub trait Storage {
    /// # Errors
    ///
    /// Will return an error if the match replay cannot be written to the storage backend
    async fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()>;
}
