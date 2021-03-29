JuAPI网关安装手册
================


## 安装

### 下载

[Windows](/releases/hyperapi-0.1-x86_64-windows)

[Linux](/releases/hyperapi-0.1-x86_64-linux)

[MacOS](/releases/hyperapi-0.1-x86_64-darwin)


### 编译

准备Rust编译环境:

```shell script
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

编译API网关:

```shell script
git clone git@github.com:leric/apihub.git 
cd apihub
git checkout v1.0
cargo build --release
cp target/release/hyperapi /usr/local/bin/
```


运行
----

使用本地配置文件运行：

```shell script
hyperapi --listen 0.0.0.0:8080  --config file:///etc/hyperapi/config.yaml
```

连接到JuAPI SaaS服务获取配置：

```shell script
hyperapi --listen 0.0.0.0:8080 --config "ws://www.juapi.cn/gw/ws/<env-access-key>"
```


