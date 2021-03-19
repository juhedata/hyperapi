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
W/Pae5ju9SFXJxIN5cLTAPf0zLKhRANCAATRnzLVO6p9GXrZras3mFBziIa/5j6r
3OGN666ZkHja+dhjnl7XAOUjQ1Legn1/CX9mkJCAbzPXbpN4izPuEaIg
-----END PRIVATE KEY-----"""
    token = gen_jwt("research/juapi-api", priv_key)
    print(token)
    headers={"Authorization": f"Bearer {token}"}
    resp = requests.get('http://10.0.49.84:8888/jwt/api/auth/me', headers=headers)
    print(resp.content)
