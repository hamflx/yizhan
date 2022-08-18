use crate::{error::YiZhanResult, serve::Serve};

pub(crate) struct YiZhanServer<S> {
    pub(crate) serve: S,
}

impl<S: Serve> YiZhanServer<S> {
    pub(crate) fn new(serve: S) -> Self {
        Self { serve }
    }

    pub(crate) async fn run(&self) -> YiZhanResult<()> {
        self.serve.run().await
    }
}
