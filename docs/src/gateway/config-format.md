网关配置文件
============

网关从服务器端获得的是所有服务和应用的配置信息。可以在JuAPI SaaS系统中的环境信息页面导出网关配置文件。


## Sample

```yaml
services:
  - service_id: leric/account_service
    path: account     # api url first segment
    protocol: http
    auth:
      type: AppKey
    timeout: 3000
    upstreams:
      - { target: "http://127.0.0.1:8000/" }
    filters:
      - type: Header
        operate_on: "request"
        injection:
          X-Authentication: "source-authenticate-token"
        removal:
          - "Authentication"
      - type: ACL
        access_control: allow
        paths:
          - methods: POST
            path_regex: /api.*
    sla:
      - name: Default
        filters:
          - type: RateLimit
            interval: 60
            limit: 100
            burst: 100
          - type: RateLimit
            interval: 1
            limit: 100
            burst: 200

clients:
  - app_key: 1345432321
    app_secret: 4535324321
    client_id: account/crm
    services:
      - leric/account_service:Default
```
