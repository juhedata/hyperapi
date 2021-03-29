使用一个API服务
==============

现在我们假设你是一名开发者，你的团队开发的一个系统需要调用其他团队提供的API服务，已经发布在JuAPI平台上。你需要了解这个API如何使用，如何认证，
接口地址，服务的性能和稳定性；接口对接完成后，需要确认接口能够满足业务调用量的吞吐量要求，响应时间要求，并能持续监控接口的调用量和性能指标；后期
在接口变更或临时维护是需要及时收到通知。

JuAPI可以帮你解决这些问题，下面我们开始吧。


## 加入JuAPI

请查看 [用户与团队](user.md)

## 创建APP

一般我们需要为每个项目创建一个APP，用来申请API服务的使用权限，这样在项目里可以用同一套app-key和app-secret配置访问这个APP申请的多个API服务。
也方便跟踪各个项目和API服务之间的依赖关系。

创建APP的表单需要填写的内容：

* 命名空间： 服务可以选择在个人或者团队下创建，团队下的服务可以有多个管理员共同管理。
* 服务名称： API名称，因为这个名称需要在URL中使用，只允许使用字母，数字，-，_这些URL中允许的字符，
        在系统中一个服务的名称会表示为<命名空间>/<服务名称>
* 应用描述： 应用描述
    
APP创建成功后会自动随机生成一组app-key和app-secret，调用API服务时将用到这两个字符串进行身份认证。

其实app-key和app-secret是一组ECDSA密钥对，服务器端只记录app-key用来校验加密签名，并不保存app-secret，只有在APP创建时会显示，请妥善保存，
如丢失可以在应用设置页面重新生成一组密钥对。


## 申请API服务

在JuAPI平台上，你需要知道发布服务的人或者团队的名称，到TA的个人页面或团队页面来找到要使用的服务。

在API服务的信息首页是API的文档，文档页右侧是申请使用服务的入口，申请使用API服务需要选择自己使用这个服务的APP，和API服务的一个SLA。有的SLA需要
等待服务管理员的审核，有些SLA是无需审核的，提交申请后就可以开始调用API服务接口了。


## 认证方式

API服务目前支持app-key和JWT两种认证方式：

### APP-KEY认证

app-key是一种简单的，但安全度较低的认证方式，只需要将app-key作为明文密码包含在HTTP请求中即可，可选的传递方式包括：

* HTTP Header: 将app-key设置在Header的X-APP-KEY字段中传递给网关
* URL Query: 在URL的Query中以_app_key为字段名，将app-key传递给网关
* URL Path: 由于有些后端接口有自己的用户认证，认证逻辑已经封装在一些代码库中不便修改，也可以直接将app-key放在URL的Path部分传递给网关，
    格式为 https://gateway.endpoint/service-path/~app-key/api-path

### JWT认证

在安全要求比较高的场景可以选择使用JWT认证方式，app-secret是一个ECDSA密钥对的PEM格式的私钥，椭圆曲线为NIST256p。app-secret只有APP的
管理员在生成密钥时可以看到，在服务器端不保存，如果丢失只能重新生成新的密钥。

在[JWT.io](https://jwt.io/)可以找到各种语言的JWT库和示例代码，这里使用的签名方式是ES256，下面是一个生成JWT的python代码实例：

```python
import jwt
from datetime import datetime, timedelta
import requests

def gen_jwt(app_id, app_secret):
    payload = {
        "sub": app_id,
        "exp": int((datetime.now() + timedelta(hours=1)).timestamp()),
        "iat": int(datetime.now().timestamp()),
        "iss": None,
    }
    token = jwt.encode(payload, app_secret, algorithm="ES256")
    return token

if __name__ == '__main__':
    priv_key = """-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgf6E6V5vuZQ9SX3VP
3OGN666ZkHja+dhjnl7XAOUjQ1Legn1/CX9mkJCAbzPXbpN4izPuEaIg
-----END PRIVATE KEY-----"""
    token = gen_jwt("research/juapi-api", priv_key)
    print(token)
    headers={"Authorization": f"Bearer {token}"}
    resp = requests.get('http://10.0.49.84:8888/jwt/api/auth/me', headers=headers)
    print(resp.content)
```


## 数据监控

在应用页面的Status标签中，可以查看从网关收集的API调用情况数据。
