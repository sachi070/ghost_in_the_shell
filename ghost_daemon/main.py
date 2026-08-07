import asyncio
from contextlib import asynccontextmanager
from dotenv import load_dotenv
from fastapi import FastAPI
load_dotenv()
from db import init_db, log_interception
from ipc_server import start_ipc_server
from llm_client import get_ai_diagnosis
from models import DiagnoseRequest, DiagnoseResponse


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Initialize SQLite schema
    init_db()

    # Spawn IPC TCP listener in background
    ipc_task = asyncio.create_task(start_ipc_server())
    yield
    ipc_task.cancel()


app = FastAPI(title="Ghost in the Shell Daemon", lifespan=lifespan)


@app.get("/health")
async def health_check():
    return {"status": "online", "daemon": "ghost-in-the-shell"}


@app.post("/diagnose", response_model=DiagnoseResponse)
async def diagnose(req: DiagnoseRequest):
    ai_result = await get_ai_diagnosis(
        command=req.command,
        exit_code=req.exit_code,
        context=req.output_context,
    )

    if ai_result:
        diagnosis = ai_result.diagnosis
        suggested_fix = ai_result.suggested_fix
        explanation = ai_result.explanation
        source = "llm"
        confidence = 0.95
    else:
        if "cat" in req.command and "No such file or directory" in req.output_context:
            missing_file = req.command.split()[-1] if len(req.command.split()) > 1 else "file"
            diagnosis = f"Target file '{missing_file}' does not exist in the current directory."
            suggested_fix = f"touch {missing_file}"
            explanation = "The cat command requires an existing file path to read."
            source = "fast_path"
            confidence = 0.85
        else:
            diagnosis = f"Command '{req.command}' failed with exit code {req.exit_code}."
            suggested_fix = "ghost doctor"
            explanation = "Fallback stub response. Verify GROQ_API_KEY in .env file."
            source = "stub"
            confidence = 0.50

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
        confidence=confidence,
        source=source,
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run("main:app", host="127.0.0.1", port=8000, reload=True)