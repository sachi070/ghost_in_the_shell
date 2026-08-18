import asyncio
from contextlib import asynccontextmanager
from dotenv import load_dotenv
from fastapi import FastAPI, Query

load_dotenv()

from db import (
    init_db,
    log_interception,
    resolve_workspace,
    get_command_recurrence,
    get_historical_context,
    get_recent_interceptions,
    search_interceptions,
    get_doctor_analytics,
)
from ipc_server import start_ipc_server
from llm_client import get_hybrid_diagnosis
from models import DiagnoseRequest, DiagnoseResponse, DoctorStatsResponse


@asynccontextmanager
async def lifespan(app: FastAPI):
    init_db()
    ipc_task = asyncio.create_task(start_ipc_server())
    yield
    ipc_task.cancel()


app = FastAPI(title="Ghost in the Shell Daemon (V2)", lifespan=lifespan)


@app.get("/health")
async def health_check():
    return {"status": "online", "daemon": "ghost-in-the-shell", "version": "2.0"}


@app.post("/diagnose", response_model=DiagnoseResponse)
async def diagnose(req: DiagnoseRequest):
    workspace = req.workspace_root or resolve_workspace(req.cwd)
    historical_notes = get_historical_context(req.command, workspace)

    # 1. Fast-Path Rules
    if "cat" in req.command and "No such file or directory" in req.output_context:
        missing_file = req.command.split()[-1] if len(req.command.split()) > 1 else "file"
        diagnosis = f"Target file '{missing_file}' does not exist in the current directory."
        suggested_fix = f"touch {missing_file}"
        explanation = "The cat command requires an existing file path to read."
        source = "fast_path"
        confidence = 0.85
    else:
        # 2. Hybrid AI Engine Query (Groq -> Ollama)
        ai_result, source = await get_hybrid_diagnosis(
            command=req.command,
            exit_code=req.exit_code,
            context=req.output_context,
            workspace=workspace,
            historical_notes=historical_notes,
        )

        if ai_result:
            diagnosis = ai_result.diagnosis
            suggested_fix = ai_result.suggested_fix
            explanation = ai_result.explanation
            confidence = 0.95 if source != "ollama" else 0.88
        else:
            diagnosis = f"Command '{req.command}' failed with exit code {req.exit_code}."
            suggested_fix = "ghost doctor"
            explanation = "No inference engine available."
            source = "stub"
            confidence = 0.50

    log_interception(
        command=req.command,
        exit_code=req.exit_code,
        diagnosis=diagnosis,
        suggested_fix=suggested_fix,
        workspace=workspace,
        engine_source=source,
    )

    recurrence = get_command_recurrence(req.command, req.exit_code, workspace)

    return DiagnoseResponse(
        diagnosis=diagnosis,
        suggested_fix=suggested_fix,
        explanation=explanation,
        confidence=confidence,
        source=source,
        workspace=workspace,
        recurrence_count=recurrence,
    )


@app.get("/history")
async def get_history(limit: int = 10, workspace: str = Query(default=None)):
    records = get_recent_interceptions(limit=limit, workspace=workspace)
    return {"status": "ok", "count": len(records), "history": records}


@app.get("/search")
async def search_history(q: str = Query(..., min_length=1), limit: int = 10):
    records = search_interceptions(query=q, limit=limit)
    return {"status": "ok", "count": len(records), "history": records}


@app.get("/stats", response_model=DoctorStatsResponse)
async def get_stats():
    stats = get_doctor_analytics()
    return DoctorStatsResponse(**stats)


if __name__ == "__main__":
    import uvicorn

    uvicorn.run("main:app", host="127.0.0.1", port=8000, reload=True)