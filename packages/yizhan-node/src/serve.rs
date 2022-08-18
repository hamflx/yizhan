use async_trait::async_trait;

use crate::error::YiZhanResult;

#[async_trait]
pub(crate) trait Serve {
    async fn run(&self) -> YiZhanResult<()>;
}
