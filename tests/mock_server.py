from typing import Optional
from fastapi import FastAPI, Request, Response, Path
from asyncio import Queue
from httpx import AsyncClient
import asyncio

app = FastAPI()
queue = Queue(maxsize=10)


@app.api_route("/api/{api:path}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def api_endpoint(req: Request, api: str):
    queue.put_nowait(req)
    return {"api": api}


@app.api_route("/error/{code}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def error_endpoint(req: Request, code: int=Path(default=500)):
    queue.put_nowait(req)
    return Response(status_code=int(code))


@app.api_route("/timeout/{seconds}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def timeout_endpoint(req: Request, seconds: int=Path(default=1)):
    queue.put_nowait(req)
    await asyncio.sleep(seconds)
    return {"sleep": seconds}


@app.get("/test")
async def run_test():
    headers = {
        "Authorization": "Bearer test-header",
        'X-APP-KEY': "1345432321",
    }
    async with AsyncClient(base_url="http://localhost:8888") as ac:
        url = "/account/api/user/hello"
        resp = await ac.get(url, headers=headers)
        print(resp.status_code)
        received = await queue.get()
        print(received.headers)
        queue.task_done()

        url = "/account/error/500"
        resp = await ac.get(url, headers=headers)
        print(resp.status_code)
        received = await queue.get()
        print(received.url)
        queue.task_done()

    return {"result": "Pass"}


if __name__ == '__main__':
    import subprocess
    import requests
    import time
    gateway = subprocess.Popen(["../target/debug/hyperapi", "--listen", "127.0.0.1:8888", "--config", "sample_config.yaml"])
    fastapi = subprocess.Popen(["uvicorn", "--port", "9999", "mock_server:app"])
    time.sleep(3)
    
    print("request test endpoint")
    resp = requests.get("http://localhost:9999/test")
    print(resp.content)

    gateway.kill()
    fastapi.kill()
