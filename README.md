# Autobahnkreuz

**Autobahnkreuz** is a [Web Application Messaging Protcol v2](http://wamp-proto.org/) router that can scale-out by hosting multiple instances that are linked through TCP streams. This router is built to be easily to deploy in a kubernetes environment.
The router aims to only implement the core WAMP profile and is planned to be later expanded with plugins to implement advanced features.
that implements most of the features defined in the advanced profile. The wampire project is written 
in [Rust](https://www.rust-lang.org/) and designed for highly concurrent asynchronous I/O. The wampire router 
provides extended functionality.  The router and client interaction with other WAMP implementations. 
Project initially forked from [wampire v0.1.0](https://github.com/ohyo-io/wampire).

> Note: This project is work-in-progress and currently only pub/sub is working. **Please do not use it in production!** The router is currently quite buggy and has a lot of glitches!

## Starting the pub/sub scenario

You can run the pub/sub scenario hosting 3 instances of the router (outside of kubernetes) on your local machine by running:

```
$ ./scripts/scenarios.sh pubsub
```

> More content coming when [`simple-raft-node`](https://github.com/fin-ger/simple-raft-node) API stabilizes.
