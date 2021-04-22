Load Test
=========

测试加入API网关对原有请求路径产生的性能影响。

测试环境为三台阿里云`ecs.sn1.medium`2CPU4G服务器，分别为Nginx挡板机，ab测试机，和API网关测试机

## Server setup

### Nginx

```bash
apt update
apt install nginx
```

### ab

```bash
apt update
apt install apache2-utils
```

### API Gateway

```bash
wget https://github.com/juhedata/hyperapi/releases/download/v0.1.0/hyperapi-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
tar zxf hyperapi-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
./hyperapi --config config.yaml --listen 0.0.0.0:9999
```

config.yaml

```yaml
services:
  - service_id: test/mws
    path: /mws
    protocol: http
    auth:
      type: AppKey
    timeout: 3
    load_balance: "load"
    error_threshold: 10
    error_reset: 60
    retry_delay: 10
    upstreams:
      - id: 1
        timeout: 3
        target: "http://192.168.0.95/"
        max_conn: 1000
    filters: []
    sla:
      - name: Default
        filters:
          - type: RateLimit
            setting:
              interval: 10
              limit: 100000
              burst: 100000

clients:
- app_key: 9cf3319cbd254202cf882a79a755ba6e
  client_id: test/client
  ip_whitelist: []
  pub_key: '-----BEGIN PUBLIC KEY-----
    MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAERxp2aXX0l2/y2y32hnk+TsJakjqd
    2DB414zO+kb1mdxM2rtq/j3WwoKEncd31UwOEMbNld/rpiP5o/sgiTUk9g==
    -----END PUBLIC KEY-----'
  services:
    test/mws: Default
```


## Nginx静态页基准测试

```
root@ab-test:~# ab -n 100000 -c 100 http://192.168.0.95/test.html
Server Software:        nginx/1.18.0
Server Hostname:        192.168.0.95
Server Port:            80

Document Path:          /test.html
Document Length:        76 bytes

Concurrency Level:      100
Time taken for tests:   5.089 seconds
Complete requests:      100000
Failed requests:        0
Total transferred:      31600000 bytes
HTML transferred:       7600000 bytes
Requests per second:    19651.91 [#/sec] (mean)
Time per request:       5.089 [ms] (mean)
Time per request:       0.051 [ms] (mean, across all concurrent requests)
Transfer rate:          6064.46 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    2   0.8      2       3
Processing:     1    4   0.7      4      10
Waiting:        0    3   0.8      3       9
Total:          2    5   0.8      5      12

Percentage of the requests served within a certain time (ms)
  50%      5
  66%      5
  75%      6
  80%      6
  90%      6
  95%      6
  98%      6
  99%      7
 100%     12 (longest request)
 ```


 ## hyperapi网关转发测试

```
root@ab-test:~# ab -n 100000 -c 100  http://192.168.0.94:9999/mws/~9cf3319cbd254202cf882a79a755ba6e/test.html
Server Software:        nginx/1.18.0
Server Hostname:        192.168.0.94
Server Port:            9999

Document Path:          /mws/~9cf3319cbd254202cf882a79a755ba6e/test.html
Document Length:        76 bytes

Concurrency Level:      100
Time taken for tests:   20.059 seconds
Complete requests:      100000
Failed requests:        0
Total transferred:      45595105 bytes
HTML transferred:       7600000 bytes
Requests per second:    4985.21 [#/sec] (mean)
Time per request:       20.059 [ms] (mean)
Time per request:       0.201 [ms] (mean, across all concurrent requests)
Transfer rate:          2219.74 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0   0.1      0       3
Processing:     4   20   3.4     20      52
Waiting:        1   20   3.4     20      52
Total:          4   20   3.4     20      52

Percentage of the requests served within a certain time (ms)
  50%     20
  66%     21
  75%     22
  80%     23
  90%     24
  95%     26
  98%     28
  99%     29
 100%     52 (longest request)
```
