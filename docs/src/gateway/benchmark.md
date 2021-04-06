Load Test
=========

测试加入API网关对原有请求路径产生的性能影响。

运行环境为两台KVM虚拟机：

    Host CPU：Intel(R) Xeon(R) CPU E5-2680 v3 @ 2.50GHz
    VCPU：4
    Memory：8G
    OS： CentOS 8

Nginx独立运行在一个虚拟机10.0.49.84，Hyperapi网关运行在虚拟机10.0.49.83，测试工具AB与网关运行在同一个服务器10.0.49.83。


直接访问Nginx静态页面：

```
[gitlab-runner@airbus ~]$ ab -n 1000000 -c 100 http://10.0.49.84/test.html

Server Software:        nginx/1.14.1
Server Hostname:        10.0.49.84
Server Port:            80

Document Path:          /test.html
Document Length:        75 bytes

Concurrency Level:      100
Time taken for tests:   60.655 seconds
Complete requests:      1000000
Failed requests:        0
Write errors:           0
Total transferred:      306000000 bytes
HTML transferred:       75000000 bytes
Requests per second:    16486.61 [#/sec] (mean)
Time per request:       6.066 [ms] (mean)
Time per request:       0.061 [ms] (mean, across all concurrent requests)
Transfer rate:          4926.66 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    2   1.1      2      12
Processing:     0    4   1.9      4      35
Waiting:        0    3   1.7      3      34
Total:          1    6   2.4      6      38

Percentage of the requests served within a certain time (ms)
  50%      6
  66%      7
  75%      7
  80%      8
  90%      9
  95%     10
  98%     12
  99%     14
 100%     38 (longest request)

```


通过Hyperapi网关访问Nginx静态页面：

```
[gitlab-runner@airbus ~]$ ab -n 1000000 -c 100 http://10.0.49.83:9999/jmeter/~099b24812494b6441562dce73d4f717e/test.html

Server Software:
Server Hostname:        10.0.49.83
Server Port:            9999

Document Path:          /jmeter/~099b24812494b6441562dce73d4f717e/test.html
Document Length:        75 bytes

Concurrency Level:      100
Time taken for tests:   102.235 seconds
Complete requests:      1000000
Failed requests:        8
   (Connect: 0, Receive: 0, Length: 8, Exceptions: 0)
Write errors:           0
Non-2xx responses:      8
Total transferred:      446947526 bytes
HTML transferred:       74999592 bytes
Requests per second:    9781.43 [#/sec] (mean)
Time per request:       10.223 [ms] (mean)
Time per request:       0.102 [ms] (mean, across all concurrent requests)
Transfer rate:          4269.32 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0   0.6      0       9
Processing:     1   10   2.6     10      35
Waiting:        1   10   2.6      9      35
Total:          1   10   2.5     10      35

Percentage of the requests served within a certain time (ms)
  50%     10
  66%     11
  75%     12
  80%     12
  90%     13
  95%     15
  98%     16
  99%     18
 100%     35 (longest request)

```


从测试数据可以看出，API网关对请求带来了大约4ms的额外延迟，对吞吐量有10%的影响，在一个4VCPU的虚拟机上，可以支撑10k的QPS。

