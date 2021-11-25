use async_trait::async_trait;

#[async_trait]
pub trait Stage<'stage> {
    async fn execute(&mut self) -> anyhow::Result<()>;
}
