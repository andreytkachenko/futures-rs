use core::pin::Pin;
use futures_core::future::Future;
use futures_core::task::{Waker, Poll};
use futures_sink::Sink;

/// Future for the [`send`](super::SinkExt::send) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct Send<'a, Si: Sink<Item> + Unpin + ?Sized, Item> {
    sink: &'a mut Si,
    item: Option<Item>,
}

// Pinning is never projected to children
impl<Si: Sink<Item> + Unpin + ?Sized, Item> Unpin for Send<'_, Si, Item> {}

impl<'a, Si: Sink<Item> + Unpin + ?Sized, Item> Send<'a, Si, Item> {
    pub(super) fn new(sink: &'a mut Si, item: Item) -> Self {
        Send {
            sink,
            item: Some(item),
        }
    }
}

impl<Si: Sink<Item> + Unpin + ?Sized, Item> Future for Send<'_, Si, Item> {
    type Output = Result<(), Si::SinkError>;

    fn poll(
        mut self: Pin<&mut Self>,
        waker: &Waker,
    ) -> Poll<Self::Output> {
        let this = &mut *self;
        if let Some(item) = this.item.take() {
            let mut sink = Pin::new(&mut this.sink);
            match sink.as_mut().poll_ready(waker) {
                Poll::Ready(Ok(())) => {
                    if let Err(e) = sink.as_mut().start_send(item) {
                        return Poll::Ready(Err(e));
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {
                    this.item = Some(item);
                    return Poll::Pending;
                }
            }
        }

        // we're done sending the item, but want to block on flushing the
        // sink
        try_ready!(Pin::new(&mut this.sink).poll_flush(waker));

        Poll::Ready(Ok(()))
    }
}
