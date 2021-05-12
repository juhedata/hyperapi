from fastapi import FastAPI, Request, Response, Path
from asyncio import Queue
import asyncio
import json
import random

app = FastAPI(debug=True)
queue = Queue(maxsize=10)


# @app.exception_handler(AssertionError)
# async def assertion_error_handler(_req: Request, exc: AssertionError):
#     print(exc)
#     return Response(
#         status_code=400,
#         content=json.dumps({"error": str(exc)}, ensure_ascii=False),
#     )
    

@app.api_route("/api/{api:path}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def api_endpoint(req: Request, api: str):
    # put request in queue, to be verified on the other side
    queue.put_nowait(req)
    return {"api": api}


@app.api_route("/error/{code}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def error_endpoint(req: Request, code: int=Path(default=500)):
    return Response(status_code=int(code))


@app.api_route("/timeout/{seconds}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def timeout_endpoint(req: Request, seconds: float=Path(default=1.0)):
    await asyncio.sleep(seconds)
    return {"sleep": seconds}


@app.api_route("/random/{seconds}", methods=['POST', 'GET', 'PUT', 'DELETE'])
async def random_delay_endpoint(req: Request, seconds: float=Path(default=1.0)):
    delay = random.random() * seconds
    await asyncio.sleep(delay)
    return {"sleep": delay}
