import asyncio
from contextlib import asynccontextmanager
from fastapi import FastAPI

from db import init_db, log_interception
from ipc_server import start_ipc_server
from models import DiagnoseRequest, DiagnoseResponse


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Initialize SQLite database
    init_db()

    # Spawn IPC server in background loop when FastAPI starts
    ipc_task = asyncio.create_task(start_ipc_server())
    yield
    ipc_task.cancel()


app = FastAPI(title="Ghost in the Shell Daemon", lifespan=lifespan)


@app.get("/health")
async def health_check():
    return {"status": "online", "daemon": "ghost-in-the-shell"}


@app.post("/diagnose", response_model=DiagnoseResponse)
async def diagnose(req: DiagnoseRequest):
    diagnosis = f"HTTP Endpoint: Command '{req.command}' failed with exit code {req.exit_code}"
    suggested_fix = "ghost doctor"
    explanation = f"Captured context for command: {req.command}"

    # Log to SQLite session database
    log_interception(
        command=req.command,
        exit_code=req.exit_code,
        diagnosis=diagnosis,
        suggested_fix=suggested_fix,
    )

    return DiagnoseResponse(
        diagnosis=diagnosis,
        suggested_fix=suggested_fix,
        explanation=explanation,
        confidence=1.0,
        source="stub",
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run("main:app", host="127.0.0.1", port=8000, reload=True)