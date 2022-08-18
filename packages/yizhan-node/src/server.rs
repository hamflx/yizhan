use crate::{error::YiZhanResult, serve::Serve};

pub(crate) struct YiZhanServer<S: Send + Sync> {
    pub(crate) serve: S,
}

impl<S: Serve + Send + Sync> YiZhanServer<S> {
    pub(crate) fn new(serve: S) -> Self {
        Self { serve }
    }

    pub(crate) async fn run(&self) -> YiZhanResult<()> {
        self.serve.run().await
    }
}
