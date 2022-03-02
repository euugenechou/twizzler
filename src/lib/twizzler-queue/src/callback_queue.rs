use std::future::Future;

use twizzler_async::{AsyncDuplex, AsyncDuplexSetup};
use twizzler_queue_raw::{QueueError, ReceiveFlags, SubmissionFlags};

use crate::Queue;

struct CallbackQueueReceiverInner<S, C> {
    queue: Queue<S, C>,
}

pub struct CallbackQueueReceiver<S, C> {
    inner: AsyncDuplex<CallbackQueueReceiverInner<S, C>>,
}

impl<S: Copy, C: Copy> AsyncDuplexSetup for CallbackQueueReceiverInner<S, C> {
    type ReadError = QueueError;
    type WriteError = QueueError;

    const READ_WOULD_BLOCK: Self::ReadError = QueueError::WouldBlock;
    const WRITE_WOULD_BLOCK: Self::WriteError = QueueError::WouldBlock;

    fn setup_read_sleep(&self) -> twizzler_abi::syscall::ThreadSyncSleep {
        self.queue.setup_read_sub_sleep()
    }

    fn setup_write_sleep(&self) -> twizzler_abi::syscall::ThreadSyncSleep {
        self.queue.setup_write_com_sleep()
    }
}

impl<S: Copy, C: Copy> CallbackQueueReceiver<S, C> {
    pub fn new(queue: Queue<S, C>) -> Self {
        Self {
            inner: AsyncDuplex::new(CallbackQueueReceiverInner { queue }),
        }
    }

    pub async fn handle<F, Fut>(&self, f: F) -> Result<(), QueueError>
    where
        F: FnOnce(u32, S) -> Fut,
        Fut: Future<Output = C>,
    {
        let (id, item) = self
            .inner
            .read_with(|inner| inner.queue.receive(ReceiveFlags::NON_BLOCK))
            .await?;
        let reply = f(id, item).await;
        self.inner
            .write_with(|inner| inner.queue.complete(id, reply, SubmissionFlags::NON_BLOCK))
            .await?;
        Ok(())
    }
}
