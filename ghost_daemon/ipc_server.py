import asyncio
import json
import os
import sys
from models import DiagnoseRequest, DiagnoseResponse

SOCKET_PATH = "/tmp/ghost.sock" if sys.platform != "win32" else "ghost.sock"
HOST = "127.0.0.1"
PORT = 9099

async def handle_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    try:
        data = await reader.read(8192)
        if not data:
            return

        payload = json.loads(data.decode("utf-8"))
        req = DiagnoseRequest(**payload)

        # Mock diagnosis logic (Replace with LLM / Rules engine in Phase 4)
        response = DiagnoseResponse(
            diagnosis=f"Command '{req.command}' failed with exit code {req.exit_code}.",
            suggested_fix=f"echo 'Retrying {req.command} with fix'",
            explanation="Captured failure via ghost-daemon IPC handler.",
            confidence=0.9
        )

        response_bytes = json.dumps(response.model_dump()).encode("utf-8")
        writer.write(response_bytes)
        await writer.drain()

    except Exception as e:
        error_resp = json.dumps({
            "diagnosis": f"Daemon error: {str(e)}",
            "suggested_fix": "none",
            "confidence": 0.0
        }).encode("utf-8")
        writer.write(error_resp)
        await writer.drain()
    finally:
        writer.close()
        await writer.wait_closed()

async def start_ipc_server():
    """Starts Unix socket server (macOS/Linux) or TCP loopback server (Windows fallback)."""
    if sys.platform != "win32":
        if os.path.exists(SOCKET_PATH):
            os.remove(SOCKET_PATH)
        server = await asyncio.start_unix_server(handle_client, path=SOCKET_PATH)
        print(f"[IPC] Listening on Unix socket: {SOCKET_PATH}")
    else:
        server = await asyncio.start_server(handle_client, HOST, PORT)
        print(f"[IPC] Listening on TCP loopback: {HOST}:{PORT}")

    async with server:
        await server.serve_forever()

if __name__ == "__main__":
    asyncio.run(start_ipc_server())