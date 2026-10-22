use parking_lot::Mutex;
use serde::Serialize;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};
use tokio::sync::Notify;

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueStatus {
    pub paused: bool,
    pub waiting: usize,
    pub running: usize,
}
struct State {
    paused: bool,
    pending: VecDeque<String>,
    active: HashSet<String>,
}
pub struct Queue {
    state: Mutex<State>,
    changed: Notify,
}
pub struct Slot {
    queue: Arc<Queue>,
    id: String,
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.queue.state.lock().active.remove(&self.id);
        self.queue.changed.notify_waiters();
    }
}
impl Queue {
    pub fn release_pending(&self, id: &str) {
        self.state.lock().pending.retain(|item| item != id);
        self.changed.notify_waiters();
    }
    pub fn new(paused: bool) -> Self {
        Self {
            state: Mutex::new(State {
                paused,
                pending: VecDeque::new(),
                active: HashSet::new(),
            }),
            changed: Notify::new(),
        }
    }
    pub fn status(&self) -> QueueStatus {
        let s = self.state.lock();
        QueueStatus {
            paused: s.paused,
            waiting: s.pending.len(),
            running: s.active.len(),
        }
    }
    pub fn set_paused(&self, paused: bool) {
        self.state.lock().paused = paused;
        self.changed.notify_waiters();
    }
    pub fn reserve(&self, id: &str) -> bool {
        let mut s = self.state.lock();
        if s.active.contains(id) || s.pending.iter().any(|v| v == id) {
            return false;
        }
        s.pending.push_back(id.to_string());
        true
    }
    pub async fn acquire(self: &Arc<Self>, id: &str, limit: usize) -> Slot {
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut s = self.state.lock();
                if !s.paused
                    && s.active.len() < limit.max(1)
                    && s.pending.front().is_some_and(|v| v == id)
                {
                    s.pending.pop_front();
                    s.active.insert(id.to_string());
                    self.changed.notify_waiters();
                    return Slot {
                        queue: self.clone(),
                        id: id.to_string(),
                    };
                }
            }
            notified.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, timeout};
    #[tokio::test]
    async fn pause_blocks_dispatch_and_resume_keeps_global_limit_and_order() {
        let queue = Arc::new(Queue::new(false));
        assert!(queue.reserve("first"));
        let first = queue.acquire("first", 1).await;
        queue.set_paused(true);
        assert!(queue.reserve("second"));
        assert!(!queue.reserve("first"));
        assert!(!queue.reserve("second"));
        drop(first);
        assert!(
            timeout(Duration::from_millis(20), queue.acquire("second", 1))
                .await
                .is_err()
        );
        queue.set_paused(false);
        let second = timeout(Duration::from_secs(1), queue.acquire("second", 1))
            .await
            .unwrap();
        queue.reserve("third");
        assert!(
            timeout(Duration::from_millis(20), queue.acquire("third", 1))
                .await
                .is_err()
        );
        drop(second);
        let third = timeout(Duration::from_secs(1), queue.acquire("third", 1))
            .await
            .unwrap();
        assert_eq!(queue.status().running, 1);
        drop(third);
        assert_eq!(queue.status().running, 0);
    }
}
