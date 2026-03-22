use crate::StreamEvent;
use anyhow::Result;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;

pub struct StreamResponse {
    receiver: mpsc::Receiver<Result<StreamEvent>>,
}

impl StreamResponse {
    pub fn new(receiver: mpsc::Receiver<Result<StreamEvent>>) -> Self {
        Self { receiver }
    }
}

impl Stream for StreamResponse {
    type Item = Result<StreamEvent>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        
        self.receiver
            .poll_recv(cx)
            .map(|opt| opt.map(|r| r.map_err(|e| anyhow::anyhow!("{}", e))))
    }
}

pub fn channel() -> (
    mpsc::Sender<Result<StreamEvent, anyhow::Error>>,
    StreamResponse,
) {
    let (tx, rx) = mpsc::channel(100);
    (tx, StreamResponse::new(rx))
}
