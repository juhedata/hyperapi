from httpx import AsyncClient
import jwt
from collections import defaultdict
from datetime import datetime
from mock_server import app, queue
import asyncio


@app.get("/test1")
async def test_appkey_service():
    print("=============TESTING MIDDLEWARES=========================")
    headers = {
        'Authorization': "toberemoved",
        'X-APP-KEY': "9cf3319cbd254202cf882a79a755ba6e",
    }
    async with AsyncClient(base_url="http://localhost:8888") as ac:
        # test header modification
        url = "/mws/api/user/hello"
        resp = await ac.get(url, headers=headers)
        assert resp.status_code == 200
        assert resp.headers.get('powered-by') == 'hyperapi'
        assert resp.headers.get('server') is None
        assert resp.headers.get('X-UPSTREAM-ID') is None
        received = await queue.get()
        request_header = received.headers
        assert request_header.get('X-TEST') == 'test-header'
        assert request_header.get('Authorization') is None
        queue.task_done()

        # test acl
        url = "/mws/api/not-found"
        resp = await ac.get(url, headers=headers)
        assert resp.status_code == 404
        assert queue.empty()  # no request received, blocked by gateway

        # test rate limit 
        url = "/mws/error/200"
        print("drain token bucket")
        for i in range(10):
            resp = await ac.get(url, headers=headers)
        resp = await ac.get(url, headers=headers)
        assert resp.status_code == 429
        print("wait token refill")
        await asyncio.sleep(3)
        for i in range(5):
            resp = await ac.get(url, headers=headers)
            assert resp.status_code == 200
        resp = await ac.get(url, headers=headers)
        assert resp.status_code == 429
        print("wait token bucket full")
        await asyncio.sleep(10)
        for i in range(10):
            resp = await ac.get(url, headers=headers)
            assert resp.status_code == 200
        resp = await ac.get(url, headers=headers)
        assert resp.status_code == 429

    return {"result": "Pass"}


@app.get("/test2")
async def test_jwt_service():
    print("=============TESTING MIDDLEWARES=========================")
    privkey = """-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgTlPYH5pUJVTlfekJ\nb5EgvrLxWo2rk+Qstt+sFJ59xvmhRANCAARHGnZpdfSXb/LbLfaGeT5OwlqSOp3Y\nMHjXjM76RvWZ3Ezau2r+PdbCgoSdx3fVTA4Qxs2V3+umI/mj+yCJNST2\n-----END PRIVATE KEY-----"""
    ts = int(datetime.now().timestamp())
    payload = {'sub': 'test/client', 'exp': ts + 3600, 'iat': ts}
    token = jwt.encode(payload, privkey, 'ES256')
    headers = {
        "Authorization": f"Bearer {token}",
    }
    async with AsyncClient(base_url="http://localhost:8888") as ac:
        print('--------------test jwt auth')
        url = "/upstream/error/400"
        resp = await ac.get(url, headers=headers)
        assert resp.status_code == 400

        print('--------------test timeout')
        url = "/upstream/timeout/4"
        resp = await ac.post(url, headers=headers)
        print(resp.content)
        assert resp.status_code == 502
        url = "/upstream/timeout/2"
        resp = await ac.put(url, headers=headers)
        assert resp.status_code == 200

        print('--------------test circuit breaker')
        url = "/upstream/error/543"
        for _i in range(3):  # trigger circurt breaker
            resp = await ac.post(url, headers=headers)  
            print(resp.headers)
        resp = await ac.post(url, headers=headers)  # CB is OPEN
        print(resp.headers)
        assert resp.status_code == 502

        print('wait retry delay, and failed')
        await asyncio.sleep(4)  # retry delay
        resp = await ac.post(url, headers=headers)
        print(resp.headers)
        assert resp.status_code == 543  
        print('go back to OPEN state')
        resp = await ac.post(url, headers=headers)
        print(resp.headers)
        assert resp.status_code == 502

        print('wait retry delay, and success')
        await asyncio.sleep(4)  # retry delay
        url = "/upstream/error/200"
        resp = await ac.post(url, headers=headers)
        print(resp.headers)
        assert resp.status_code == 200  # change to CLOSE state

        url = "/upstream/error/543"
        resp = await ac.post(url, headers=headers)
        assert resp.status_code == 543

        print('--------------test concurrent limit')
        url = "/upstream/timeout/2"
        reqs = [ac.get(url, headers=headers) for i in range(20)]
        resps = await asyncio.gather(*reqs)
        print([r.content for r in resps])
        assert len([s for s in resps if s.status_code == 200]) == 10
        assert len([s for s in resps if s.status_code == 502]) == 10

    return {"result": "Pass"}


@app.get("/test3")
async def test_load_balance():
    print("=============TESTING LOAD BALANCE=========================")
    headers = {
        'X-APP-KEY': "9cf3319cbd254202cf882a79a755ba6e",
        'X-LB-HASH': "test",
    }
    async with AsyncClient(base_url="http://localhost:8888") as ac:
        print('------------test random lb------------')
        url = "/lb1/error/200"
        counter = defaultdict(int)
        for i in range(100):
            resp = await ac.get(url, headers=headers)
            upstream = resp.headers.get('x-upstream-id')
            counter[upstream] += 1
        print(counter)
        print("load distribution should be around 10:1")
        
        print('------------test hash lb------------')
        url = "/lb2/error/200"
        counter = defaultdict(int)
        for i in range(100):
            resp = await ac.get(url, headers=headers)
            upstream = resp.headers.get('x-upstream-id')
            counter[upstream] += 1
        print(counter)
        assert len(counter) == 1
        print("load distribution should be around 10:1")

        print('------------test connection based lb------------')
        url = "/lb_conn"
        concurrent = [runner(ac, url, headers, 50) for i in range(10)]
        counters = await asyncio.gather(*concurrent)
        counter = defaultdict(list)
        for c in counters:
            for usid in c.keys():
                counter[usid].extend(c[usid])
        print([(x, len(counter[x]), sum(counter[x]), sum(counter[x])/len(counter[x]))
                for x in counter.keys()])

        print('------------test latency based lb------------')
        url = "/lb_load"
        counter = await runner(ac, url, headers, 100)
        print([(x, len(counter[x]), sum(counter[x]), sum(counter[x])/len(counter[x]))
                for x in counter.keys()])

    return {"result": "Pass"}

async def runner(ac, url, headers, counts):
    counter = counter = defaultdict(list)
    for i in range(counts):
        start = datetime.now().timestamp()
        resp = await ac.get(url, headers=headers)
        end = datetime.now().timestamp()
        upstream = resp.headers.get('x-upstream-id')
        counter[upstream].append(end - start)
    return counter

def run_test():
    import subprocess
    import requests
    import time

    gateway_port = 54321
    gateway = subprocess.Popen(["../target/debug/hyperapi", "--listen", f"127.0.0.1:{gateway_port}", "--config", "sample_config.yaml"])
    mock_port = 54320
    fastapi = subprocess.Popen(["uvicorn", "--port", f"{mock_port}", "gateway_test:app"])
    time.sleep(3)
    
    try:
        print("request test endpoint, middleware test, appkey auth")
        resp = requests.get(f"http://localhost:{mock_port}/test1")
        assert resp.status_code == 200

        print("request test endpoint, upstream test, jwt auth")
        resp = requests.get(f"http://localhost:{mock_port}/test2")
        assert resp.status_code == 200

        print("request test endpoint, load balance test, appkey auth")
        resp = requests.get(f"http://localhost:{mock_port}/test3")
        assert resp.status_code == 200
    finally:
        gateway.kill()
        fastapi.kill()


if __name__ == '__main__':
    run_test()

