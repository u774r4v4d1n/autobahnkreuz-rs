extern crate autobahnkreuz;

use autobahnkreuz::router::Router;
extern crate env_logger;

extern crate argparse;
use argparse::{ArgumentParser, Store};

fn main() {
    env_logger::init();

    let mut port = "8090".to_string();
    {  // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("Options");
        ap.refer(&mut port)
            .add_option(&["-P", "--port"], Store,
                        "Listening port");
        ap.parse_args_or_exit();
    }

    let router = Router::new();
    let addr = format!("0.0.0.0:{}", port);
    let child = router.listen(addr.as_str());
    child.join().unwrap();
}
