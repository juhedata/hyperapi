Deployment
===========


## 安装MySQL

```shell script
wget https://cdn.mysql.com//Downloads/MySQL-8.0/mysql-8.0.23-1.el7.x86_64.rpm-bundle.tar
tar xf mysql-8.0.23-1.el7.x86_64.rpm-bundle.tar
yum install mysql-community-{server,client,common,libs}-*
systemctl start mysqld
```

从安装日志中找出临时的root密码，登陆mysql并修改密码：
    
    sudo grep 'temporary password' /var/log/mysqld.log
    mysql -uroot -p
    > ALTER USER 'root'@'localhost' IDENTIFIED BY 'T@A7rmE313@$IM2G';  

修改mysql配置 /etc/my.cnf：

    innodb_buffer_pool_size = 4G
    innodb_flush_method = O_DIRECT
    innodb_file_per_table = 1
    innodb_flush_log_at_trx_commit = 0

创建数据库：
    
    CREATE DATABASE juapi default charset utf8mb4 COLLATE utf8mb4_general_ci;
    
创建用户：

    CREATE USER 'juapi'@'%' IDENTIFIED BY 'Covid#yqfk2o21';
    GRANT ALL PRIVILEGES ON juapi.* TO 'juapi'@'%';
    FLUSH PRIVILEGES;
    
设置主从复制 

On Master:
 
    修改/etc/my.cnf，添加：
    log_bin = mysql-bin
    server_id = 11
    重启mysqld服务
 
    INSTALL PLUGIN rpl_semi_sync_master SONAME 'semisync_master.so';
    INSTALL PLUGIN rpl_semi_sync_slave SONAME 'semisync_slave.so';
    
    CREATE USER 'replica'@'%' IDENTIFIED BY 'MS#replica1';
    GRANT replication slave ON *.* TO 'replica'@'%';
    ALTER USER 'replica'@'%' IDENTIFIED WITH mysql_native_password BY 'MS#replica1';
    FLUSH PRIVILEGES;
    SHOW MASTER STATUS;
    
On Slave:

    修改/etc/my.cnf，添加：
    log_bin = mysql-bin
    server_id = 10
    log_slave_updates = 1
    重启mysqld服务
    
    INSTALL PLUGIN rpl_semi_sync_master SONAME 'semisync_master.so';
    INSTALL PLUGIN rpl_semi_sync_slave SONAME 'semisync_slave.so';

    CHANGE MASTER TO master_host='172.25.227.11', master_user='replica',
        master_password='MS#replica1', master_log_file='mysql-bin.000001',
        master_log_pos=608;
        
    START SLAVE;
    
    
## Python环境


Setup Python Env

```shell script
sudo yum install @development zlib-devel bzip2 bzip2-devel readline-devel sqlite sqlite-devel openssl-devel xz xz-devel libffi-devel findutils git vim openldap-devel
curl -L https://github.com/pyenv/pyenv-installer/raw/master/bin/pyenv-installer | bash
pyenv install 3.8.7
```

上面的curl命令可能无法下载，可以在浏览器中打开文件内容，粘贴到服务器上的文本文件，在bash中执行。

```shell script
cd /opt
git clone git@codebowl.juhe.cn:tianju/tianju-saas.git tianju
cd tianju
pyenv virtualenv 3.8.7 tianju
pyenv activate tianju
pip install -r requirements.txt
```

配置Systemd，添加配置文件 /etc/systemd/system/tianju.service

```
[Unit]
Description=Tianju SaaS
After=network.target

[Service]
PIDFile=/run/tianju.pid
User=root
Group=root
WorkingDirectory=/opt/tianju
ExecStart=/root/.pyenv/versions/tianju/bin/gunicorn --pid /run/tianju.pid \
--access-logfile /var/log/tianju_access.log --error-logfile /var/log/tianju_error.log \
--bind 0.0.0.0:5050 -w 4 -k uvicorn.workers.UvicornWorker manage:app
ExecReload=/bin/kill -s HUP $MAINPID
ExecStop=/bin/kill -s TERM $MAINPID
PrivateTmp=true

[Install]
WantedBy=multi-user.target

```

## 部署目录说明

    DEPLOY_ROOT/
        current/        JuAPI SaaS系统Python代码目录
        env             当前环境的.env文件
        tianju.pid      gunicorn的PID文件
        log/            gunicorn配置的日志存放路径
        metrics/        fastapi多进程共享的metrics数据存放路径
        public/         前端代码路径，Nginx中配置的Web主目录
            *upload -> ../upload
            *releases -> ../releases
            *book -> ../current/docs/book
        upload/         头像上传存放路径
        releases/       存放预编译的网关可执行文件，供下载
        prometheus_targets/     保存prometheus target配置文件，由SaaS服务生成，prometheus读取


## MDBook

```shell script
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install mdbook
```

## 配置Web服务


```shell script
wget https://nginx.org/download/nginx-1.18.0.tar.gz
tar zxf nginx-1.18.0.tar.gz
cd nginx-1.18.0
./configure 
make
sudo make install
```

配置Nginx, root指向前端代码部署的目录：

```
server {
    listen 80;
    server_name www.juapi.cn;
    index index.html;
    root /opt/tianju/public;

    location / {
        # ln -s  /opt/tianju/current/docs/book /opt/tianju/public/book
        # ln -s  /opt/tianju/upload /opt/tianju/public/upload
        # ln -s  /opt/tianju/releases /opt/tianju/public/releases
        try_files $uri $uri/ /index.html;
        location ~ .*\.(gif|jpg|jpeg|png|bmp|swf|mp3)$ {
            expires  10d;
        }
        location ~ .*\.(js|css)?$ {
            expires  12h;
        }
    }
    location /api/ {
        proxy_pass http://127.0.0.1:9000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
    location /ws/ {
        proxy_pass http://127.0.0.1:9000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
    }
}
```
    
配置日志清理 

/etc/logrotate.d/nginx
```
/usr/local/nginx/logs/*log {
    create 0644 root root
    daily
    rotate 10
    missingok
    notifempty
    compress
    sharedscripts
    postrotate
        /bin/kill -USR1 `cat /usr/local/nginx/logs/nginx.pid 2>/dev/null` 2>/dev/null || true
    endscript
}
```

/etc/logrotate.d/tianju
```
/var/logs/tianju_*log {
    create 0644 root root
    daily
    rotate 10
    missingok
    notifempty
    compress
    sharedscripts
    postrotate
        /bin/kill -USR1 `cat /run/tianju.pid 2>/dev/null` 2>/dev/null || true
    endscript
}
```


## 配置Prometheus

各个网关通过file_sd_configs添加到prometheus的监控目标中，prometheus.yml配置文件中添加一下设置：

    scrape_configs:
      - job_name: 'tinaju-gateway'
        file_sd_configs:
          - files:
              - "/opt/tianju/prometheus_targets/*.yaml"
            refresh_interval: 10m

