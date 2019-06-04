use crate::router::{ConnectionHandler, ConnectionState};
use ws::{CloseCode, Handler, Message as WSMessage,
         Request, Response, Result as WSResult};

use crate::messages::{ErrorDetails, ErrorType, Message, Reason};
use rmp_serde::Deserializer as RMPDeserializer;
use serde_json;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Cursor;
use crate::{Error, ErrorKind, WampResult, ID};

impl ConnectionHandler {
    pub fn send_message(&self, message: Message) {
        self.router.send_message(self.info_id, message)
    }

    fn handle_message(&mut self, message: Message) -> WampResult<()> {
        log::debug!("Received message {:?}", message);
        match message {
            Message::Hello(realm, details) => {
                self.handle_hello(realm, details)?;
            },
            Message::Subscribe(request_id, options, topic) => {
                self.handle_subscribe(request_id, options, topic);
            }
            Message::Publish(request_id, options, topic, args, kwargs) => {
                self.handle_publish(request_id, options, topic, args, kwargs);
            }
            Message::Unsubscribe(request_id, topic_id) => {
                self.handle_unsubscribe(request_id, topic_id);
            }
            Message::Goodbye(details, reason) => {
                self.handle_goodbye(details, reason)?;
            },
            Message::Register(_request_id, _options, _procedure) => {
                unimplemented!();
            }
            Message::Unregister(_request_id, _procedure_id) => {
                unimplemented!();
            }
            Message::Call(_request_id, _options, _procedure, _args, _kwargs) => {
                unimplemented!();
            }
            Message::Yield(_invocation_id, _options, _args, _kwargs) => {
                unimplemented!();
            }
            Message::Error(_e_type, _request_id, _details, _reason, _args, _kwargs) => {
                unimplemented!();
            }
            t => Err(Error::new(ErrorKind::InvalidMessageType(t)))?,
        }

        Ok(())
    }

    fn parse_message(&self, msg: WSMessage) -> WampResult<Message> {
        match msg {
            WSMessage::Text(payload) => match serde_json::from_str(&payload) {
                Ok(message) => Ok(message),
                Err(e) => Err(Error::new(ErrorKind::JSONError(e))),
            },
            WSMessage::Binary(payload) => {
                let mut de = RMPDeserializer::new(Cursor::new(payload));
                match Deserialize::deserialize(&mut de) {
                    Ok(message) => Ok(message),
                    Err(e) => Err(Error::new(ErrorKind::MsgPackError(e))),
                }
            }
        }
    }

    fn send_error(&self, err_type: ErrorType, request_id: ID, reason: Reason) {
        self.send_message(Message::Error(err_type, request_id, HashMap::new(), reason, None, None));
    }

    fn send_abort(&self, reason: Reason) {
        self.send_message(Message::Abort(ErrorDetails::new(), reason));
    }

    fn on_message_error(&self, error: Error) -> WSResult<()> {
        use std::error::Error as StdError;
        match error.get_kind() {
            ErrorKind::WSError(e) => Err(e)?,
            ErrorKind::URLError(_) => unimplemented!(),
            ErrorKind::HandshakeError(r) => {
                log::error!("Handshake error: {}", r);
                self.send_abort(r);
                self.terminate_connection()?;
            }
            ErrorKind::UnexpectedMessage(msg) => {
                log::error!("Unexpected Message: {}", msg);
                self.terminate_connection()?;
            }
            ErrorKind::ThreadError(_) => unimplemented!(),
            ErrorKind::ConnectionLost => unimplemented!(),
            ErrorKind::Closing(_) => {
                unimplemented!{}
            }
            ErrorKind::JSONError(e) => {
                log::error!("Could not parse JSON: {}", e);
                self.terminate_connection()?;
            }
            ErrorKind::MsgPackError(e) => {
                log::error!("Could not parse MsgPack: {}", e.description());
                self.terminate_connection()?;
            }
            ErrorKind::MalformedData => unimplemented!(),
            ErrorKind::InvalidMessageType(msg) => {
                log::error!("Router unable to handle message {:?}", msg);
                self.terminate_connection()?;
            }
            ErrorKind::InvalidState(s) => {
                log::error!("Invalid State: {}", s);
                self.terminate_connection()?;
            }
            ErrorKind::Timeout => {
                log::error!("Connection timeout");
                self.terminate_connection()?;
            }
            ErrorKind::ErrorReason(err_type, id, reason) => self.send_error(err_type, id, reason),
        }
        Ok(())
    }
}

impl Handler for ConnectionHandler {
    fn on_request(&mut self, request: &Request) -> WSResult<Response> {
        log::info!("New request");
        let mut response = match Response::from_request(request) {
            Ok(response) => response,
            Err(e) => {
                log::error!("Could not create response: {}", e);
                return Err(e);
            }
        };
        self.process_protocol(request, &mut response)?;
        log::debug!("Sending response");
        Ok(response)
    }

    fn on_message(&mut self, msg: WSMessage) -> WSResult<()> {
        log::debug!("Receveied message: {:?}", msg);
        let message = match self.parse_message(msg) {
            Err(e) => return self.on_message_error(e),
            Ok(m) => m,
        };
        match self.handle_message(message) {
            Err(e) => self.on_message_error(e),
            _ => Ok(()),
        }
    }

    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        let state = self.router
                .connection(self.info_id)
                .lock()
                .unwrap()
                .state.clone();
        if state != ConnectionState::Disconnected {
            log::trace!("Client disconnected.  Closing connection");
            self.terminate_connection().ok();
        }
    }
}
