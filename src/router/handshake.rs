use super::{ConnectionHandler, ConnectionState, WAMP_JSON, WAMP_MSGPACK};

use ws::{CloseCode, Error as WSError, ErrorKind as WSErrorKind, Request, Response,
         Result as WSResult};

use crate::messages::{ErrorDetails, HelloDetails, Message, Reason, RouterRoles, WelcomeDetails, URI};
use crate::{Error, ErrorKind, WampResult};

impl ConnectionHandler {
    pub fn handle_hello(&self, realm: URI, _details: HelloDetails) -> WampResult<()> {
        log::debug!("Responding to hello message (realm: {:?})", realm);
        self.set_realm(realm.uri)?;
        self.router.set_state(self.info_id, ConnectionState::Connected);
        self.send_message(Message::Welcome(self.info_id, WelcomeDetails::new(RouterRoles::new())));
        Ok(())
    }

    pub fn handle_goodbye(&self, _details: ErrorDetails, reason: Reason) -> WampResult<()> {
        if let Ok(info) = self.info() {
            let state = info.lock().unwrap().state.clone();
            match state {
                ConnectionState::Initializing => {
                    // TODO check specification for how this ought to work.
                    Err(Error::new(ErrorKind::InvalidState(
                        "Received a goodbye message before handshake complete",
                    )))
                }
                ConnectionState::Connected => {
                    log::info!("Received goodbye message with reason: {:?}", reason);
                    self.remove();
                    self.send_message(Message::Goodbye(ErrorDetails::new(), Reason::GoodbyeAndOut));
                    self.router.set_state(self.info_id, ConnectionState::Disconnected);
                    let senders = self.router.senders.lock().unwrap();
                    let sender = senders.get(&self.info_id).unwrap();
                    match sender.close(CloseCode::Normal) {
                        Err(e) => Err(Error::new(ErrorKind::WSError(e))),
                        _ => Ok(()),
                    }
                }
                ConnectionState::ShuttingDown => {
                    log::info!(
                        "Received goodbye message in response to our goodbye message with reason: {:?}",
                        reason
                    );
                    self.router.set_state(self.info_id, ConnectionState::Disconnected);
                    let senders = self.router.senders.lock().unwrap();
                    let sender = senders.get(&self.info_id).unwrap();
                    match sender.close(CloseCode::Normal) {
                        Err(e) => Err(Error::new(ErrorKind::WSError(e))),
                        _ => Ok(()),
                    }
                }
                ConnectionState::Disconnected => {
                    log::warn!("Received goodbye message after closing connection");
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    fn set_realm(&self, realm: String) -> WampResult<()> {
        log::debug!("Setting realm to {}", realm);
        if realm == "default" {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::HandshakeError(Reason::NoSuchRealm)))
        }
    }

    pub fn process_protocol(&self, request: &Request, response: &mut Response) -> WSResult<()> {
        log::debug!("Checking protocol");
        let protocols = request.protocols()?;
        for protocol in protocols {
            if protocol == WAMP_JSON || protocol == WAMP_MSGPACK {
                response.set_protocol(protocol);
                self.router.set_protocol(self.info_id, protocol.to_string());
                return Ok(());
            }
        }
        Err(WSError::new(
            WSErrorKind::Protocol,
            format!(
                "Neither {} nor {} were selected as Websocket sub-protocols",
                WAMP_JSON, WAMP_MSGPACK
            ),
        ))
    }
}
