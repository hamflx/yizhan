use crate::{error::Result, serve::Serve};

pub(crate) struct YiZhanServer<S: Serve> {
    pub(crate) serve: S,
}

impl<S: Serve> YiZhanServer<S> {
    pub(crate) fn new(serve: S) -> Self {
        Self { serve }
    }

    pub(crate) async fn run(&self) -> Result<()> {
        self.serve.run().await
    }
}
