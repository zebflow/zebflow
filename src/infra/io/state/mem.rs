//! In-memory [`StateBus`] implementation backed by the existing [`crate::infra::mem::MemHub`].

use serde_json::Value;

use crate::infra::mem::MemHub;

use super::interface::{
    StateBus, StateBusCapabilities, StateBusError, StateBusStats, StateSubscription,
};

/// `StateBus` adapter over the existing in-process `MemHub`.
#[derive(Clone, Default)]
pub struct MemStateBus {
    hub: MemHub,
}

impl MemStateBus {
    /// Create a new in-memory state bus.
    pub fn new() -> Self {
        Self { hub: MemHub::new() }
    }

    /// Wrap an existing `MemHub`.
    pub fn from_hub(hub: MemHub) -> Self {
        Self { hub }
    }

    /// Borrow the inner `MemHub`.
    pub fn hub(&self) -> &MemHub {
        &self.hub
    }
}

impl StateBus for MemStateBus {
    fn id(&self) -> &'static str {
        "state.mem"
    }

    fn capabilities(&self) -> StateBusCapabilities {
        StateBusCapabilities {
            ttl: true,
            atomic_incr: true,
            pubsub: true,
            durable: false,
            shared_coordination: false,
        }
    }

    fn stats(&self) -> Result<StateBusStats, StateBusError> {
        let stats = self.hub.stats();
        Ok(StateBusStats {
            implementation: self.id().to_string(),
            entry_count: stats.entry_count,
            channel_count: stats.channel_count,
            subscriber_count: stats.subscriber_count,
        })
    }

    fn get(&self, owner: &str, project: &str, key: &str) -> Result<Option<Value>, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.get(owner, project, key))
    }

    fn set(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        value: Value,
        ttl_secs: Option<u64>,
    ) -> Result<(), StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        self.hub.set(owner, project, key, value, ttl_secs);
        Ok(())
    }

    fn exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.exists(owner, project, key))
    }

    fn expire(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        ttl_secs: Option<u64>,
    ) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.expire(owner, project, key, ttl_secs))
    }

    fn incr(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        amount: i64,
    ) -> Result<i64, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.incr(owner, project, key, amount))
    }

    fn del(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.del(owner, project, key))
    }

    fn publish(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        message: Value,
    ) -> Result<usize, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_channel(channel)?;
        Ok(self.hub.publish(owner, project, channel, message))
    }

    fn subscribe(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<StateSubscription, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_channel(channel)?;
        Ok(self.hub.subscribe(owner, project, channel))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::infra::io::state::StateBus;

    use super::MemStateBus;

    #[tokio::test]
    async fn enforces_validation_and_exposes_stats() {
        let bus = MemStateBus::new();
        bus.set(
            "superadmin",
            "default",
            "session:user@example.com",
            json!({"ok": true}),
            Some(60),
        )
        .expect("set");
        let stats = bus.stats().expect("stats");
        assert_eq!(stats.entry_count, 1);
        assert!(
            bus.set("bad owner", "default", "safe", json!(1), None)
                .is_err()
        );
        assert!(
            bus.publish("superadmin", "default", "bad channel", json!(1))
                .is_err()
        );
    }

    #[tokio::test]
    async fn supports_full_mem_feature_surface() {
        let bus = MemStateBus::new();
        bus.set("superadmin", "default", "counter", json!(1), Some(1))
            .expect("set");
        assert_eq!(
            bus.get("superadmin", "default", "counter").expect("get"),
            Some(json!(1))
        );
        assert!(
            bus.exists("superadmin", "default", "counter")
                .expect("exists")
        );
        assert_eq!(
            bus.incr("superadmin", "default", "counter", 2)
                .expect("incr"),
            3
        );
        assert!(
            bus.expire("superadmin", "default", "counter", None)
                .expect("expire")
        );
        assert!(bus.del("superadmin", "default", "counter").expect("del"));
        assert!(
            !bus.exists("superadmin", "default", "counter")
                .expect("exists after del")
        );
    }

    #[tokio::test]
    async fn publish_and_subscribe_are_project_scoped() {
        let bus = MemStateBus::new();
        let mut rx = bus
            .subscribe("superadmin", "default", "events/orders")
            .expect("subscribe");
        bus.publish("superadmin", "default", "events/orders", json!({"id": 42}))
            .expect("publish");
        let message = rx.recv().await.expect("message");
        assert_eq!(message, json!({"id": 42}));

        let mut other = bus
            .subscribe("superadmin", "other", "events/orders")
            .expect("other subscribe");
        let _ = bus
            .publish("superadmin", "default", "events/orders", json!({"id": 43}))
            .expect("publish again");
        let message = rx.recv().await.expect("second message");
        assert_eq!(message, json!({"id": 43}));
        assert!(
            other.try_recv().is_err(),
            "other project must not receive the message"
        );
    }
}
