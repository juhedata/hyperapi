JuAPI网关安装手册
================


## 安装

### 下载

可以在Github下载预编译的二进制文件：

https://github.com/juhedata/hyperapi/releases


### 编译

准备Rust编译环境:

```shell script
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

编译安装API网关:

```shell script
cargo install hyperapi
```


运行
----

使用本地配置文件运行：

```shell script
hyperapi --listen 0.0.0.0:9999  --config file:///etc/hyperapi/config.yaml
```

连接到JuAPI SaaS服务获取配置：

```shell script
hyperapi --listen 0.0.0.0:9999 --config "ws://www.juapi.cn/gw/ws/<env-access-key>"
```

启用HTTPS：

```
hyperapi --listen 0.0.0.0:443 --config "ws://www.juapi.cn/gw/ws/<env-access-key>" --cert_file cert_file.pem --key_file private_key.pem
```
