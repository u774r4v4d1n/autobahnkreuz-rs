extern crate autobahnkreuz;

use autobahnkreuz::router::Router;
extern crate env_logger;

use std::net::ToSocketAddrs;

fn main() {
    env_logger::init();

    let wamp_address = std::env::var("WAMP_ADDRESS").ok()
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
        }).expect("Please specify a WAMP_ADDRESS (domain:port) via an environment variable!");

    let router = Router::new();
    let child = router.listen(wamp_address);
    child.join().unwrap();
}
