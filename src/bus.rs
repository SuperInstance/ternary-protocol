use crate::message::TernaryMessage;

/// How a message should be routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingMode {
    Unicast,
    Multicast,
    Broadcast,
}

/// Errors from message bus operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusError {
    /// Agent ID not registered.
    AgentNotFound(u64),
    /// No subscribers for the given topic.
    NoSubscribers,
    /// Queue full for agent.
    QueueFull(u64),
}

/// A simple in-process message bus supporting unicast, multicast, and broadcast routing.
#[derive(Debug)]
pub struct MessageBus {
    /// Per-agent message queues: agent_id -> inbox.
    queues: std::collections::HashMap<u64, Vec<TernaryMessage>>,
    /// Multicast group memberships: group_id -> set of agent_ids.
    groups: std::collections::HashMap<u64, Vec<u64>>,
}

impl MessageBus {
    /// Create a new empty message bus.
    pub fn new() -> Self {
        MessageBus {
            queues: std::collections::HashMap::new(),
            groups: std::collections::HashMap::new(),
        }
    }

    /// Register an agent on the bus.
    pub fn register(&mut self, agent_id: u64) {
        self.queues.entry(agent_id).or_insert_with(Vec::new);
    }

    /// Unregister an agent from the bus.
    pub fn unregister(&mut self, agent_id: u64) {
        self.queues.remove(&agent_id);
    }

    /// Add an agent to a multicast group.
    pub fn join_group(&mut self, group_id: u64, agent_id: u64) {
        self.groups.entry(group_id).or_insert_with(Vec::new);
        if let Some(members) = self.groups.get_mut(&group_id) {
            if !members.contains(&agent_id) {
                members.push(agent_id);
            }
        }
    }

    /// Remove an agent from a multicast group.
    pub fn leave_group(&mut self, group_id: u64, agent_id: u64) {
        if let Some(members) = self.groups.get_mut(&group_id) {
            members.retain(|&id| id != agent_id);
        }
    }

    /// Send a message using the specified routing mode.
    pub fn send(&mut self, msg: TernaryMessage, mode: RoutingMode) -> Result<(), BusError> {
        match mode {
            RoutingMode::Unicast => {
                let receiver = msg.receiver;
                if !self.queues.contains_key(&receiver) {
                    return Err(BusError::AgentNotFound(receiver));
                }
                self.queues.get_mut(&receiver).unwrap().push(msg);
                Ok(())
            }
            RoutingMode::Broadcast => {
                for (_, queue) in self.queues.iter_mut() {
                    queue.push(msg.clone());
                }
                Ok(())
            }
            RoutingMode::Multicast => {
                let group_id = msg.receiver;
                let members = self.groups.get(&group_id)
                    .ok_or(BusError::NoSubscribers)?;
                if members.is_empty() {
                    return Err(BusError::NoSubscribers);
                }
                for &agent_id in members {
                    if let Some(queue) = self.queues.get_mut(&agent_id) {
                        queue.push(msg.clone());
                    }
                }
                Ok(())
            }
        }
    }

    /// Receive all pending messages for an agent (drains the inbox).
    pub fn receive(&mut self, agent_id: u64) -> Result<Vec<TernaryMessage>, BusError> {
        if !self.queues.contains_key(&agent_id) {
            return Err(BusError::AgentNotFound(agent_id));
        }
        Ok(self.queues.get_mut(&agent_id).unwrap().drain(..).collect())
    }

    /// Number of registered agents.
    pub fn agent_count(&self) -> usize {
        self.queues.len()
    }

    /// Pending message count for an agent.
    pub fn pending_count(&self, agent_id: u64) -> Option<usize> {
        self.queues.get(&agent_id).map(|q| q.len())
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::Payload;
    use crate::trit::Trit;

    fn make_msg(id: u64, sender: u64, receiver: u64) -> TernaryMessage {
        TernaryMessage::new(MessageId(id), sender, receiver, Payload::from_trits(&[Trit::Pos]), id * 100)
    }

    #[test]
    fn bus_unicast() {
        let mut bus = MessageBus::new();
        bus.register(1);
        bus.register(2);
        bus.send(make_msg(1, 1, 2), RoutingMode::Unicast).unwrap();
        assert_eq!(bus.pending_count(1), Some(0));
        assert_eq!(bus.pending_count(2), Some(1));
        let msgs = bus.receive(2).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, MessageId(1));
    }

    #[test]
    fn bus_unicast_unknown() {
        let mut bus = MessageBus::new();
        bus.register(1);
        let err = bus.send(make_msg(1, 1, 99), RoutingMode::Unicast);
        assert_eq!(err, Err(BusError::AgentNotFound(99)));
    }

    #[test]
    fn bus_broadcast() {
        let mut bus = MessageBus::new();
        bus.register(1);
        bus.register(2);
        bus.register(3);
        bus.send(make_msg(1, 1, 0), RoutingMode::Broadcast).unwrap();
        assert_eq!(bus.receive(1).unwrap().len(), 1);
        assert_eq!(bus.receive(2).unwrap().len(), 1);
        assert_eq!(bus.receive(3).unwrap().len(), 1);
    }

    #[test]
    fn bus_multicast() {
        let mut bus = MessageBus::new();
        bus.register(1);
        bus.register(2);
        bus.register(3);
        bus.join_group(10, 1);
        bus.join_group(10, 3);
        // receiver field = group_id for multicast
        bus.send(make_msg(1, 2, 10), RoutingMode::Multicast).unwrap();
        assert_eq!(bus.receive(1).unwrap().len(), 1);
        assert_eq!(bus.receive(2).unwrap().len(), 0);
        assert_eq!(bus.receive(3).unwrap().len(), 1);
    }

    #[test]
    fn bus_unregister() {
        let mut bus = MessageBus::new();
        bus.register(1);
        bus.unregister(1);
        assert_eq!(bus.agent_count(), 0);
        assert_eq!(bus.receive(1), Err(BusError::AgentNotFound(1)));
    }
}
