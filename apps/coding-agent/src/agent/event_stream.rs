use futures::channel::mpsc;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct EventStream<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> EventStream<T> {
    pub fn new() -> (Self, mpsc::Sender<T>) {
        let (tx, rx) = mpsc::channel(100);
        (EventStream { receiver: rx }, tx)
    }
}

impl<T: Send> Stream for EventStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.receiver.poll_next_unpin(cx)
    }
}
