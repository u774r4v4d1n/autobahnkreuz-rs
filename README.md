<p align="center">
  <img src="assets/autobahnkreuz.svg" height="256"><br>
</p>
<h1 align="center">Welcome to Autobahnkreuz 👋</h1>
<p align="center">
  <a href="https://travis-ci.org/verkehrsministerium/autobahnkreuz-rs">
    <img src="https://travis-ci.org/verkehrsministerium/autobahnkreuz-rs.svg?branch=master">
  </a>
  <a href="https://hub.docker.com/r/fin1ger/autobahnkreuz-rs">
    <img alt="Docker Pulls" src="https://img.shields.io/docker/pulls/fin1ger/autobahnkreuz-rs">
  </a>
  <a href="https://hub.docker.com/r/fin1ger/autobahnkreuz-rs/tags">
    <img alt="Container Size" src="https://img.shields.io/microbadger/image-size/fin1ger/autobahnkreuz-rs">
  </a>
  <a href="https://hub.docker.com/r/fin1ger/autobahnkreuz-rs/tags">
    <img alt="Container Layers" src="https://img.shields.io/microbadger/layers/fin1ger/autobahnkreuz-rs">
  </a>
  <a href="https://github.com/verkehrsministerium/autobahnkreuz-rs/blob/master/LICENSE">
    <img alt="GitHub" src="https://img.shields.io/github/license/verkehrsministerium/autobahnkreuz-rs.svg">
  </a>
  <a href="http://spacemacs.org">
    <img src="https://cdn.rawgit.com/syl20bnr/spacemacs/442d025779da2f62fc86c2082703697714db6514/assets/spacemacs-badge.svg" />
  </a>
  <a href="http://makeapullrequest.com">
    <img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" target="_blank" />
  </a>
  <br>
  <i>Rust implementation of a distributed, cloud-native WAMP (Web Application Messaging Protocol) router</i>
</p>

---

> Note: This project is work-in-progress and currently only pub/sub is working. **Please do not use it in production!** The router is currently quite buggy and has a lot of glitches!

## What is this?

**Autobahnkreuz** is a [Web Application Messaging Protcol v2](http://wamp-proto.org/) router that can scale-out by hosting multiple instances that are linked through TCP streams. This router is built with easy kubernetes deployment in mind. The router aims to only implement the core WAMP profile and is planned to be later expanded with plugins to implement advanced features.

The project was initially forked from [wampire v0.1.0](https://github.com/ohyo-io/wampire).

## How to use

You can run the pub/sub scenario hosting 3 instances of the router (outside of kubernetes) on your local machine by running:

```
$ ./scripts/scenarios.sh pubsub
```

> More content coming when [`simple-raft-node`](https://github.com/fin-ger/simple-raft-node) API stabilizes.

## Scientific Research

> 📄 The [`Autobahnkreuz Paper`](https://github.com/fin-ger/building-a-distributed-wamp-router/releases)

## Authors

**Fin Christensen**

> [:octocat: `@fin-ger`](https://github.com/fin-ger)  
> [:elephant: `@fin_ger@mastodon.social`](https://mastodon.social/web/accounts/787945)  
> [:bird: `@fin_ger_github`](https://twitter.com/fin_ger_github)  

## Show your support

Give a :star: if this project helped you!
