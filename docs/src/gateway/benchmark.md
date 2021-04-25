Load Test
=========

测试加入API网关对原有请求路径产生的性能影响。

测试环境为三台阿里云`ecs.c7.large`2CPU4G服务器，分别为Nginx挡板机，ab测试机，和API网关测试机

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


## Nginx静态页基准测试

```
root@hyperapi01:~# ab -n 500000 -c 100  http://192.168.0.49/test.html
This is ApacheBench, Version 2.3 <$Revision: 1843412 $>
Copyright 1996 Adam Twiss, Zeus Technology Ltd, http://www.zeustech.net/
Licensed to The Apache Software Foundation, http://www.apache.org/

Server Software:        nginx/1.18.0
Server Hostname:        192.168.0.49
Server Port:            80

Document Path:          /test.html
Document Length:        27 bytes

Concurrency Level:      100
Time taken for tests:   12.901 seconds
Complete requests:      500000
Failed requests:        0
Total transferred:      133500000 bytes
HTML transferred:       13500000 bytes
Requests per second:    38756.59 [#/sec] (mean)
Time per request:       2.580 [ms] (mean)
Time per request:       0.026 [ms] (mean, across all concurrent requests)
Transfer rate:          10105.48 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    1   0.2      1       2
Processing:     1    2   0.5      2      31
Waiting:        0    1   0.5      1      31
Total:          1    3   0.4      3      32

Percentage of the requests served within a certain time (ms)
  50%      3
  66%      3
  75%      3
  80%      3
  90%      3
  95%      3
  98%      3
  99%      3
 100%     32 (longest request)
 ```


 ## hyperapi网关转发测试

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
    upstreams:
      - id: 1
        timeout: 3
        target: "http://192.168.0.49/"
        max_conn: 1000
        weight: 100
        error_threshold: 10
        error_reset: 60
        retry_delay: 10
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

Install hyperapi and startup

```bash
cargo install hyperapi
hyperapi --config config.yaml --listen 0.0.0.0:9999
```


Test Result

```
root@hyperapi01:~# ab -n 500000 -c 200  http://192.168.0.48:9999/mws/~9cf3319cbd254202cf882a79a755ba6e/test.html
This is ApacheBench, Version 2.3 <$Revision: 1843412 $>
Copyright 1996 Adam Twiss, Zeus Technology Ltd, http://www.zeustech.net/
Licensed to The Apache Software Foundation, http://www.apache.org/

Server Software:        nginx/1.18.0
Server Hostname:        192.168.0.48
Server Port:            9999

Document Path:          /mws/~9cf3319cbd254202cf882a79a755ba6e/test.html
Document Length:        27 bytes

Concurrency Level:      200
Time taken for tests:   49.196 seconds
Complete requests:      500000
Failed requests:        0
Total transferred:      203475090 bytes
HTML transferred:       13500000 bytes
Requests per second:    10163.46 [#/sec] (mean)
Time per request:       19.678 [ms] (mean)
Time per request:       0.098 [ms] (mean, across all concurrent requests)
Transfer rate:          4039.08 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0  11.9      0    1017
Processing:     2   19   2.4     19     213
Waiting:        1   19   2.4     19     213
Total:          4   20  12.2     20    1042

Percentage of the requests served within a certain time (ms)
  50%     20
  66%     20
  75%     21
  80%     21
  90%     22
  95%     23
  98%     25
  99%     26
 100%   1042 (longest request)
```


## kong

config

```yaml
_format_version: "2.1"
_transform: true
services:
- name: test-service
  url: http://192.168.0.49
  plugins:
  - name: key-auth
  - name: rate-limiting
    config:
      second: 20000
      policy: local
  routes:
  - name: my-route
    paths:
    - /test/

consumers:
- username: my-user
  keyauth_credentials:
  - key: my-key
```

Test Result

```
root@hyperapi01:~# ab -n 500000 -c 200 -H 'apikey: my-key' http://192.168.0.48:8000/test/test.html
This is ApacheBench, Version 2.3 <$Revision: 1843412 $>
Copyright 1996 Adam Twiss, Zeus Technology Ltd, http://www.zeustech.net/
Licensed to The Apache Software Foundation, http://www.apache.org/

Server Software:        nginx/1.18.0
Server Hostname:        192.168.0.48
Server Port:            8000

Document Path:          /test/test.html
Document Length:        27 bytes

Concurrency Level:      200
Time taken for tests:   52.094 seconds
Complete requests:      500000
Failed requests:        0
Total transferred:      247247589 bytes
HTML transferred:       13500000 bytes
Requests per second:    9598.10 [#/sec] (mean)
Time per request:       20.837 [ms] (mean)
Time per request:       0.104 [ms] (mean, across all concurrent requests)
Transfer rate:          4634.98 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0   0.1      0       6
Processing:     0   21  19.2     20      88
Waiting:        0   21  19.2     20      88
Total:          0   21  19.2     21      88

Percentage of the requests served within a certain time (ms)
  50%     21
  66%     36
  75%     37
  80%     38
  90%     42
  95%     47
  98%     57
  99%     68
 100%     88 (longest request)
```


## EOLinker Goku

开源版本文档严重缺失，基本不可用，未成功运行测试。


## APISIX

按照[官网文档](https://apisix.apache.org/docs/apisix/getting-started)，使用docker安装了APISIX网关和APISIX Dashboard.

配置转发到内网Nginx静态页面的服务，开启了apikey认证，和limit-req插件。

```
root@hyperapi01:~# ab -n 500000 -c 200  -H 'apikey: abcdefg'  http://192.168.0.48:9080/test/test.htmlThis is ApacheBench, Version 2.3 <$Revision: 1843412 $>
Copyright 1996 Adam Twiss, Zeus Technology Ltd, http://www.zeustech.net/
Licensed to The Apache Software Foundation, http://www.apache.org/

Server Software:        APISIX/2.3
Server Hostname:        192.168.0.48
Server Port:            9080

Document Path:          /test/test.html
Document Length:        27 bytes

Concurrency Level:      200
Time taken for tests:   53.273 seconds
Complete requests:      500000
Failed requests:        0
Total transferred:      135500000 bytes
HTML transferred:       13500000 bytes
Requests per second:    9385.67 [#/sec] (mean)
Time per request:       21.309 [ms] (mean)
Time per request:       0.107 [ms] (mean, across all concurrent requests)
Transfer rate:          2483.90 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0   0.8      0      10
Processing:     0   21  20.7     14      73
Waiting:        0   21  20.7     14      73
Total:          0   21  20.6     17      74

Percentage of the requests served within a certain time (ms)
  50%     17
  66%     41
  75%     42
  80%     43
  90%     44
  95%     45
  98%     46
  99%     46
 100%     74 (longest request)
 ```

