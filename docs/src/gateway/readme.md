技术设计
========

对于承载数据流量的API网关，是企业的核心基础设施，高性能，高并发，稳定，安全是基本的要求，而Rust语言是完成这个工作最合适的工具：

* 和C/C++相同的[性能和内存使用效率](https://benchmarksgame-team.pages.debian.net/benchmarksgame/fastest/rust.html)
* 通过强大的类型系统和所有权检查保证内存安全和线程安全
* 零成本抽象的异步计算可以轻松高效的应对高并发

此项目使用Rust语言开发，基于Tokio异步计算框架。


## 代码模块

### GatewayServer

负责创建并管理各个子模块，和创建RequestHandler。


### ConfigSource

负责监听配置来源的更新消息，每一种配置来源的实现都是一个ConfigUpdate消息的Stream，在GatewayServer中会将ConfigSource中取得的配置
更新消息通过广播通知给AuthService和各个Middleware。


### AuthService

负责对请求进行身份认证，确认当前请求的ServiceID，ClientID和SLA。


### Middleware

Middleware和AuthService都是使用Actor模式实现的，通过Channel接收请求和返回结果。


### ProxyHandler

负责转发API请求到上游服务
