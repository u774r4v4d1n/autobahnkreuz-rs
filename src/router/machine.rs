use simple_raft_node::{
    apply,
    retrieve,
    Machine,
    MachineCore,
    MachineCoreError,
    RequestManager,
    RequestError,
};
use crate::router::{
    RouterInfo,
    RouterCore,
    ConnectionInfo,
    ConnectionState,
    MatchingPolicy,
    URI,
    SubscriptionPatternNode,
    Message,
    WAMP_JSON,
};
use ws::{Message as WSMessage, Result as WSResult, Sender};

use crate::utils::StructMapWriter;
use failure::Backtrace;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rmp_serde::Serializer;
use futures::executor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouterChange {
    ShutdownSender {
        connection_id: u64,
    },
    SetState {
        connection_id: u64,
        state: ConnectionState,
    },
    SetProtocol {
        connection_id: u64,
        protocol: String,
    },
    AddConnection {
        connection_id: u64,
    },
    RemoveConnection {
        connection_id: u64,
    },
    AddSubscription {
        connection_id: u64,
        request_id: u64,
        topic: URI,
        matching_policy: MatchingPolicy,
    },
    RemoveSubscription {
        connection_id: u64,
        subscription_id: u64,
        request_id: u64,
    },
    // TODO: don't make this a state-change as it does not actually change the state
    //       instead, just tell the other nodes to send this message if they own the connection
    SendMessage {
        message: Message,
        connection_id: u64,
        protocol: String,
    },
}

#[derive(Debug, Clone)]
pub enum RouterProperty {
    Subscriptions,
    Connections,
    Connection {
        connection_id: u64,
    },
    TopicId {
        topic: URI,
    },
}

#[derive(Debug, Clone)]
pub enum RouterPropertyValue {
    Subscriptions(Arc<Mutex<SubscriptionPatternNode<u64>>>),
    Connections(Arc<Mutex<HashMap<u64, Arc<Mutex<ConnectionInfo>>>>>),
    Connection(Arc<Mutex<ConnectionInfo>>),
    TopicId(u64),
}

impl MachineCore for RouterCore {
    type StateChange = RouterChange;
    type StateIdentifier = RouterProperty;
    type StateValue = RouterPropertyValue;

    fn serialize(&self) -> Result<Vec<u8>, MachineCoreError> {
        unimplemented!();
    }

    fn deserialize(&mut self, _data: Vec<u8>) -> Result<(), MachineCoreError> {
        unimplemented!();
    }

    fn apply(&mut self, state_change: RouterChange) {
        log::debug!("foooooooooooooooo");
        //log::debug!("got a state change {:?}", state_change);
        match state_change {
            RouterChange::SendMessage { connection_id, protocol, message } => {
                log::trace!("sending message {:?} to {}", message, connection_id);
                self.send_message(connection_id, protocol, message).ok();
            },
            RouterChange::ShutdownSender { connection_id } => {
                log::trace!("shutting down sender of connection {}", connection_id);
                self.shutdown_sender(&connection_id);
            },
            RouterChange::SetState { connection_id, state } => {
                log::trace!("setting state of connection {} to {:?}", connection_id, state);
                self.set_state(connection_id, state);
            },
            RouterChange::SetProtocol { connection_id, protocol } => {
                log::trace!("setting protocol of connection {} to {}", connection_id, protocol);
                self.set_protocol(connection_id, protocol);
            },
            RouterChange::AddConnection { connection_id } => {
                log::trace!("adding connection {}", connection_id);
                self.add_connection(connection_id);
            },
            RouterChange::RemoveConnection { connection_id } => {
                log::trace!("removing connection {}", connection_id);
                self.remove_connection(connection_id);
            },
            RouterChange::AddSubscription { connection_id, request_id, topic, matching_policy } => {
                log::trace!(
                    "adding subscription for topic {:?} on connection {}",
                    topic,
                    connection_id,
                );
                self.add_subscription(connection_id, request_id, topic, matching_policy).ok();
            },
            RouterChange::RemoveSubscription { connection_id, subscription_id, request_id } => {
                log::trace!(
                    "removing subscription {} from connection {}",
                    subscription_id,
                    connection_id,
                );
                self.remove_subscription(&connection_id, &subscription_id, &request_id).ok();
            },
        }
    }

    fn retrieve(&self, state_identifier: RouterProperty) -> Result<RouterPropertyValue, RequestError> {
        match state_identifier {
            RouterProperty::Subscriptions => {
                Ok(RouterPropertyValue::Subscriptions(self.subscription_manager.subscriptions.clone()))
            },
            RouterProperty::Connections => {
                Ok(RouterPropertyValue::Connections(self.connections.clone()))
            },
            RouterProperty::Connection { connection_id } => {
                self.connections.lock().unwrap().get(&connection_id)
                    .ok_or(RequestError::StateRetrieval(Backtrace::new()))
                    .map(|c| RouterPropertyValue::Connection(c.clone()))
            },
            RouterProperty::TopicId { topic } => {
                self.subscription_manager.subscription_ids_to_uris.iter()
                    .filter(|(_, uri)| uri.0 == topic.uri)
                    .next()
                    .ok_or(RequestError::StateRetrieval(Backtrace::new()))
                    .map(|(id, _)| RouterPropertyValue::TopicId(*id))
            },
        }
    }
}

impl Machine for RouterInfo {
    type Core = RouterCore;

    fn init(&mut self, request_manager: RequestManager<RouterCore>) {
        log::debug!("initializing router-raft-machine...");
        self.request_manager = Some(request_manager);
    }

    fn core(&self) -> RouterCore {
        RouterCore::default()
    }
}

impl RouterInfo {
    pub fn shutdown_sender(&self, id: &u64) {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(manager, RouterChange::ShutdownSender { connection_id: *id }))
                .expect("failed to shutdown sender");
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn set_state(&self, connection_id: u64, state: ConnectionState) {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(manager, RouterChange::SetState { connection_id, state }))
                .expect("failed to set connection state")
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn set_protocol(&self, connection_id: u64, protocol: String) {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(manager, RouterChange::SetProtocol { connection_id, protocol }))
                .expect("failed to set connection protocol");
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn add_connection(&self, connection_id: u64, sender: Sender) {
        self.senders.lock().unwrap().insert(connection_id, sender);

        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(manager, RouterChange::AddConnection { connection_id }))
                .expect("failed to add connection");
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn remove_connection(&self, connection_id: u64) {
        self.senders.lock().unwrap().remove(&connection_id);

        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(manager, RouterChange::RemoveConnection { connection_id }))
                .expect("failed to remove connection");
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn remove_subscription(
        &self,
        connection_id: u64,
        subscription_id: u64,
        request_id: u64,
    ) {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(
                manager,
                RouterChange::RemoveSubscription {
                    connection_id,
                    subscription_id,
                    request_id,
                },
            )).expect("failed to remove subscription");
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn add_subscription(
        &self,
        connection_id: u64,
        request_id: u64,
        topic: URI,
        matching_policy: MatchingPolicy,
    ) {
        log::debug!(
            "machine is proposing to add subscription ({}, {}, {:?}, {:?})",
            connection_id,
            request_id,
            topic,
            matching_policy,
        );
        if let Some(ref manager) = self.request_manager {
            executor::block_on(apply(
                manager,
                RouterChange::AddSubscription {
                    connection_id,
                    request_id,
                    topic,
                    matching_policy,
                },
            )).expect("failed to add subscription");
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn subscriptions(&self) -> Arc<Mutex<SubscriptionPatternNode<u64>>> {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(retrieve(manager, RouterProperty::Subscriptions))
                .and_then(|res| match res {
                    RouterPropertyValue::Subscriptions(subscriptions) => Ok(subscriptions),
                    _ => Err(RequestError::StateRetrieval(Backtrace::new())),
                })
                .expect("failed to retrieve subscriptions")
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn connections(&self) -> Arc<Mutex<HashMap<u64, Arc<Mutex<ConnectionInfo>>>>> {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(retrieve(manager, RouterProperty::Connections))
                .and_then(|res| match res {
                    RouterPropertyValue::Connections(connections) => Ok(connections),
                    _ => Err(RequestError::StateRetrieval(Backtrace::new())),
                })
                .expect("failed to retrieve connections")
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn connection(&self, connection_id: u64) -> Arc<Mutex<ConnectionInfo>> {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(retrieve(manager, RouterProperty::Connection { connection_id }))
                .and_then(|res| match res {
                    RouterPropertyValue::Connection(connection) => Ok(connection),
                    _ => Err(RequestError::StateRetrieval(Backtrace::new())),
                })
                .expect("failed to retrieve connection")
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn topic_id(&self, topic: URI) -> u64 {
        if let Some(ref manager) = self.request_manager {
            executor::block_on(retrieve(manager, RouterProperty::TopicId { topic }))
                .and_then(|res| match res {
                    RouterPropertyValue::TopicId(topic) => Ok(topic),
                    _ => Err(RequestError::StateRetrieval(Backtrace::new())),
                })
                .expect("failed to retrieve topic id")
        } else {
            panic!("router is not initialized");
        }
    }

    pub fn send_message(&self, connection_id: u64, message: Message) {
        log::debug!("bar");
        let arc = self.connection(connection_id);
        let connection = arc.lock().unwrap();
        log::debug!("bar2");
        if let Some(sender) = self.senders.lock().unwrap().get(&connection_id) {
            log::debug!("Sending message {:?} via {}", message, connection.protocol);
            let send_result = if connection.protocol == WAMP_JSON {
                send_message_json(sender, &message)
            } else {
                send_message_msgpack(sender, &message)
            };
            send_result.expect("failed to send message");
        } else if let Some(ref manager) = self.request_manager {
            log::debug!("bar3");
            executor::block_on(apply(manager, RouterChange::SendMessage {
                connection_id,
                message,
                protocol: connection.protocol.clone(),
            })).expect("failed to send message");
            log::debug!("bar4");
        } else {
            panic!("router is not initialized");
        }
    }
}

pub fn send_message_json(sender: &Sender, message: &Message) -> WSResult<()> {
    // Send the message
    sender.send(WSMessage::Text(serde_json::to_string(message).unwrap()))
}

pub fn send_message_msgpack(sender: &Sender, message: &Message) -> WSResult<()> {
    // Send the message
    let mut buf: Vec<u8> = Vec::new();
    message
        .serialize(&mut Serializer::with(&mut buf, StructMapWriter))
        .unwrap();
    sender.send(WSMessage::Binary(buf))
}
