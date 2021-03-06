mod patterns;
use super::{random_id, ConnectionHandler};

use crate::messages::{EventDetails, Message, PublishOptions, SubscribeOptions, URI};
pub use crate::router::pubsub::patterns::SubscriptionPatternNode;
use crate::{Dict, List, MatchingPolicy };

impl ConnectionHandler {
    pub fn handle_subscribe(
        &mut self,
        request_id: u64,
        options: SubscribeOptions,
        topic: URI,
    ) {
        log::debug!(
            "Responding to subscribe message (conn: {}, id: {}, topic: {})",
            self.info_id, request_id, topic.uri
        );
        let topic_id = random_id();
        self.subscribed_topics.push((topic_id, request_id));
        self.router.add_subscription(
            self.info_id,
            request_id,
            topic.clone(),
            options.pattern_match,
            topic_id,
            random_id(),
        );
    }

    pub fn handle_unsubscribe(&mut self, request_id: u64, subscription_id: u64) {
        self.router.remove_subscription(self.info_id, subscription_id, request_id);
        self.subscribed_topics.retain(|(tid, _rid)| *tid != subscription_id);
    }

    pub fn handle_publish(
        &mut self,
        request_id: u64,
        options: PublishOptions,
        topic: URI,
        args: Option<List>,
        kwargs: Option<Dict>,
    ) {
        log::debug!(
            "Responding to publish message (id: {}, topic: {})",
            request_id, topic.uri
        );
        let publication_id = random_id();
        let mut event_message = Message::Event(
            1,
            publication_id,
            EventDetails::new(),
            args.clone(),
            kwargs.clone(),
        );
        let my_id = self.info_id;
        for (subscriber_id, topic_id, policy) in self.subscriptions.lock().unwrap()
            .filter(topic.clone())
        {
            if *subscriber_id != my_id {
                log::debug!("publishing to {} with id {}", subscriber_id, topic_id);
                if let Message::Event(
                    ref mut old_topic,
                    ref _publish_id,
                    ref mut details,
                    ref _args,
                    ref _kwargs,
                ) = event_message {
                    *old_topic = topic_id;
                    details.topic = if policy == MatchingPolicy::Strict {
                        None
                    } else {
                        Some(topic.clone())
                    };
                }

                self.router.send_message(*subscriber_id, event_message.clone());
            }
        }

        if options.should_acknowledge() {
            self.send_message(Message::Published(request_id, publication_id));
        }
    }
}
