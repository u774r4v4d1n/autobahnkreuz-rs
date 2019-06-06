mod handshake;
mod messaging;
mod pubsub;
mod machine;

use crate::messages::{ErrorDetails, Message, Reason, URI};
use rand::distributions::{Distribution, Range};
use rand::thread_rng;
use crate::router::pubsub::SubscriptionPatternNode;
use crate::router::machine::send_message_json;
use crate::router::machine::send_message_msgpack;
use std::collections::HashMap;
use std::marker::Sync;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::env;
use ws::{listen as ws_listen, Result as WSResult, Sender};
use simple_raft_node::{RequestManager, Node, Config, transports::TcpConnectionManager, storages::MemStorage};
use regex::Regex;
use crate::{ID, Error, ErrorType, ErrorKind, MatchingPolicy, WampResult};
use serde::{Serialize, Deserialize};
use std::net::ToSocketAddrs;

#[derive(Debug, Clone, Default)]
struct SubscriptionManager {
    subscriptions: Arc<Mutex<SubscriptionPatternNode<u64>>>,
    subscription_ids_to_uris: HashMap<u64, (String, bool)>,
}

pub struct Router {
    node: Node<RouterInfo>,
}

#[derive(Debug, Clone)]
pub struct RouterCore {
    subscription_manager: SubscriptionManager,
    connections: Arc<Mutex<HashMap<u64, Arc<Mutex<ConnectionInfo>>>>>,
    senders: Arc<Mutex<HashMap<u64, Sender>>>,
}

#[derive(Debug, Clone, Default)]
struct RouterInfo {
    request_manager: Option<RequestManager<RouterCore>>,
    senders: Arc<Mutex<HashMap<u64, Sender>>>,
}

struct ConnectionHandler {
    info_id: u64,
    router: RouterInfo,
    subscribed_topics: Vec<ID>,
}

#[derive(Debug)]
pub struct ConnectionInfo {
    state: ConnectionState,
    protocol: String,
    id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Initializing,
    Connected,
    ShuttingDown,
    Disconnected,
}

unsafe impl Sync for Router {}

static WAMP_JSON: &'static str = "wamp.2.json";
static WAMP_MSGPACK: &'static str = "wamp.2.msgpack";

fn random_id() -> u64 {
    let mut rng = thread_rng();
    // TODO make this a constant
    let between = Range::new(0, 1u64.rotate_left(56) - 1);
    between.sample(&mut rng)
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    #[inline]
    pub fn new() -> Router {
        let node_id_msg = "Please specify a NODE_ID >= 0 via an environment variable!";
        let re = Regex::new(r"\d+").unwrap();
        let node_id_str = env::var("NODE_ID").expect(node_id_msg);
        let mut node_id = re.find(node_id_str.as_str())
            .expect(node_id_msg)
            .as_str()
            .parse::<u64>()
            .expect(node_id_msg);

        node_id += 1;

        let node_address = env::var("NODE_ADDRESS").ok()
        .map(|address| {
            loop {
                log::info!("Trying to resolve binding IP address {}...", address);
                match address.to_socket_addrs() {
                    Ok(mut addr) => {
                        return addr
                            .next()
                            .expect("The binding address does not resolve to a valid IP or port!");
                    },
                    Err(e) => {
                        log::warn!("Could not resolve binding address {}: {}", address, e);
                    },
                }
            }
        }).expect("Please specify a NODE_ADDRESS (domain:port) via an environment variable!");

        let gateway = env::var("NODE_GATEWAY").ok()
            .map(|gateway| {
                loop {
                    log::info!("Trying to resolve gateway address {}...", gateway);
                    match gateway.to_socket_addrs() {
                        Ok(mut addr) => {
                            return addr
                                .next()
                                .expect("The gateway address does not resolve to a valid IP or port!");
                        },
                        Err(e) => {
                            log::warn!("Could not resolve gateway {}: {}", gateway, e);
                        },
                    }
                }
            }).expect("The gateway address environment variable NODE_GATEWAY is not specified!");

        let config = Config {
            id: node_id,
            tag: format!("node_{}", node_id),
            election_tick: 10,
            heartbeat_tick: 3,
            ..Default::default()
        };
        let machine = RouterInfo::default();
        let storage = MemStorage::new();
        let mgr = TcpConnectionManager::new(node_address).unwrap();
        let node = Node::new(
            config,
            gateway,
            machine,
            storage,
            mgr,
        );
        let stop = node.stop_handler();

        ctrlc::set_handler(move || {
            stop();

            // wait until stop is finished
            thread::sleep(Duration::from_micros(200));

            std::process::exit(0);
        }).expect("error setting Ctrl-C handler");

        Router {
            node,
        }
    }

    pub fn listen<A>(&self, url: A) -> JoinHandle<()>
    where
        A: ToSocketAddrs + std::fmt::Debug + Send + Sync + 'static
    {
        let router_info = self.node.machine().clone();
        thread::spawn(move || {
            ws_listen(url, |sender| {
                let id = random_id();
                router_info.add_connection(id, sender);
                ConnectionHandler {
                    info_id: id,
                    subscribed_topics: Vec::new(),
                    router: router_info.clone(),
                }
            }).unwrap();
        })
    }

    pub fn shutdown(&self) {
        let info = self.node.machine();
        let arc = info.connections();
        let connections = arc.lock().unwrap();
        for id in connections.keys() {
            info.send_message(
                *id,
                Message::Goodbye(ErrorDetails::new(), Reason::SystemShutdown),
            );
            info.set_state(*id, ConnectionState::ShuttingDown);
        }
        log::info!("Goodbye messages sent.  Waiting 5 seconds for response");
        thread::sleep(Duration::from_secs(5));
        for id in connections.keys() {
            info.shutdown_sender(id);
        }
    }
}

impl RouterCore {
    pub fn send_message(
        &self,
        connection_id: u64,
        protocol: String,
        message: Message,
    ) -> WampResult<()> {
        log::debug!("handling send_message");
        if let Some(sender) = self.senders.lock().unwrap().get(&connection_id) {
            log::debug!("Sending message {:?} via {}", message, protocol);
            let send_result = if protocol == WAMP_JSON {
                send_message_json(sender, &message)
            } else {
                send_message_msgpack(sender, &message)
            };
            log::debug!("sending succeeded");
            match send_result {
                Ok(()) => Ok(()),
                Err(e) => Err(Error::new(ErrorKind::WSError(e))),
            }
        } else {
            log::debug!("connection {} is not on this node, dropping message", connection_id);
            Ok(())
        }
    }

    pub fn shutdown_sender(&self, id: &u64) {
        if let Some(sender) = self.senders.lock().unwrap().get_mut(id) {
            sender.shutdown().ok();
        }
    }

    pub fn set_state(&self, connection_id: u64, state: ConnectionState) {
        let connections = self.connections.lock().unwrap();
        let connection = connections.get(&connection_id).unwrap();
        connection.lock().unwrap().state = state;
    }

    pub fn set_protocol(&self, connection_id: u64, protocol: String) {
        let connections = self.connections.lock().unwrap();
        let connection = connections.get(&connection_id).unwrap();
        connection.lock().unwrap().protocol = protocol;
    }

    pub fn add_connection(&mut self, connection_id: u64) {
        self.connections.lock().unwrap().insert(connection_id, Arc::new(Mutex::new(ConnectionInfo {
            state: ConnectionState::Initializing,
            protocol: String::new(),
            id: connection_id,
        })));
    }

    pub fn remove_connection(&mut self, connection_id: u64) {
        self.connections.lock().unwrap().remove(&connection_id);
    }

    pub fn remove_subscription(&mut self, connection_id: &u64, subscription_id: &u64, request_id: &u64) -> WampResult<()> {
        if let Some(&(ref topic_uri, is_prefix)) =
            self.subscription_manager.subscription_ids_to_uris.get(subscription_id)
        {
            log::trace!("Removing subscription to {:?}", topic_uri);
            self.subscription_manager
                .subscriptions
                .lock().unwrap()
                .unsubscribe_with(topic_uri, connection_id, is_prefix)
                .map_err(|e| Error::new(ErrorKind::ErrorReason(
                    ErrorType::Unsubscribe,
                    *request_id,
                    e.reason(),
                )))?;
            log::trace!("Subscription tree: {:?}", self.subscription_manager.subscriptions);
        } else {
            return Err(Error::new(ErrorKind::ErrorReason(
                ErrorType::Unsubscribe,
                *request_id,
                Reason::NoSuchSubscription,
            )));
        }
        let connections = self.connections.lock().unwrap();
        let connection = connections.get(connection_id).unwrap().lock().unwrap();
        self.send_message(
            *connection_id,
            connection.protocol.clone(),
            Message::Unsubscribed(*request_id),
        )?;

        Ok(())
    }

    pub fn add_subscription(
        &mut self,
        connection_id: u64,
        request_id: u64,
        topic: URI,
        matching_policy: MatchingPolicy,
        id: ID,
        prefix_id: ID,
    ) -> WampResult<()> {
        log::debug!(
            "machine is adding subscription ({}, {}, {:?}, {:?})",
            connection_id,
            request_id,
            topic,
            matching_policy,
        );
        let topic_id = match self.subscription_manager.subscriptions.lock().unwrap().subscribe_with(
            &topic,
            connection_id,
            matching_policy,
            id,
            prefix_id,
        ) {
            Ok(topic_id) => topic_id,
            Err(e) => {
                return Err(Error::new(ErrorKind::ErrorReason(
                    ErrorType::Subscribe,
                    request_id,
                    e.reason(),
                )))
            }
        };
        log::debug!("subscription for {} on {} got id {}", topic.uri, connection_id, topic_id);
        self.subscription_manager.subscription_ids_to_uris.insert(
            topic_id,
            (topic.uri, matching_policy == MatchingPolicy::Prefix),
        );
        let connections = self.connections.lock().unwrap();
        let connection = connections.get(&connection_id).unwrap().lock().unwrap();
        self.send_message(
            connection_id,
            connection.protocol.clone(),
            Message::Subscribed(request_id, topic_id),
        )?;

        Ok(())
    }
}

impl ConnectionHandler {
    fn remove(&self) {
        log::trace!(
            "Removing subscriptions for client {}",
            self.info_id,
        );
        for subscription_id in &self.subscribed_topics {
            log::trace!("Looking for subscription {}", subscription_id);
            self.router.remove_subscription(self.info_id, 0, *subscription_id);
            self.router.remove_connection(self.info_id);
        }
    }

    fn terminate_connection(&self) -> WSResult<()> {
        self.remove();
        Ok(())
    }

    fn info(&self) -> Arc<Mutex<ConnectionInfo>> {
        self.router.connection(self.info_id).clone()
    }
}
