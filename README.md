JuAPI Gateway
==============

[![Build Status](https://img.shields.io/github/workflow/status/juhedata/hyperapi/Rust?style=flat-square)](https://github.com/juhedata/hyperapi/actions?workflow=Rust)
[![License](https://img.shields.io/crates/l/hyperapi?style=flat-square)](https://crates.io/crates/hyperapi)
[![crates.io](https://img.shields.io/crates/v/hyperapi?style=flat-square)](https://crates.io/crates/hyperapi)


A simple and performant API gateway work with JuAPI SaaS (Or use with static config file).


## Features

* Client authentication (AppKey, JWT)
* Load balancing (weighted, connections, latency, hash)
* Circuit breaker
* Request rate limit
* Header modification
* API path access control
* Client-wise service level control
* Online realtime config update (file, websocket, etcd)
* Prometheus metrics
* HTTPS support


## Roadmap

* support k8s config source, work as an ingress
* integrate logging facility
* support canary deployment