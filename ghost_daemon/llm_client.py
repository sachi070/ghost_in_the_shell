import json
import os
import re
from typing import Optional, Tuple
import httpx
from pydantic import BaseModel


class LLMDiagnosis(BaseModel):
    diagnosis: str
    suggested_fix: str
    explanation: str


SYSTEM_PROMPT = """You are Ghost in the Shell, an expert CLI error diagnostician.
Analyze terminal command failure context and return a JSON object with:
1. "diagnosis": A concise 1-sentence root-cause summary.
2. "suggested_fix": The EXACT, single executable CLI command to fix or address the issue.
3. "explanation": A 1-2 sentence detailed breakdown.

Rules:
- Output ONLY raw valid JSON with keys "diagnosis", "suggested_fix", and "explanation".
- "suggested_fix" must be a clean, copy-pasteable CLI command without markdown fences.
"""


def _clean_json_output(raw_text: str) -> Optional[dict]:
    try:
        return json.loads(raw_text)
    except Exception:
        # Regex fallback in case models wrap output with markdown backticks
        match = re.search(r"\{.*\}", raw_text, re.DOTALL)
        if match:
            try:
                return json.loads(match.group(0))
            except Exception:
                pass
    return None


async def _query_groq_or_openrouter(
    prompt: str, api_key: str
) -> Optional[LLMDiagnosis]:
    if api_key.startswith("gsk_") or os.getenv("GROQ_API_KEY"):
        endpoint = "https://api.groq.com/openai/v1/chat/completions"
        default_model = "llama-3.3-70b-versatile"
    else:
        endpoint = "https://openrouter.ai/api/v1/chat/completions"
        default_model = "google/gemini-2.5-flash"

    model = os.getenv("GHOST_LLM_MODEL", default_model)
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prompt},
        ],
        "response_format": {"type": "json_object"},
        "temperature": 0.1,
    }

    async with httpx.AsyncClient(timeout=6.0) as client:
        resp = await client.post(endpoint, headers=headers, json=payload)
        if resp.status_code == 200:
            content = resp.json()["choices"][0]["message"]["content"]
            parsed = _clean_json_output(content)
            if parsed:
                return LLMDiagnosis(
                    diagnosis=parsed.get("diagnosis", "Command execution failed."),
                    suggested_fix=parsed.get("suggested_fix", "echo 'No fix available'"),
                    explanation=parsed.get("explanation", ""),
                )
    return None


async def _query_local_ollama(prompt: str) -> Optional[LLMDiagnosis]:
    ollama_host = os.getenv("OLLAMA_HOST", "http://127.0.0.1:11434")
    model = os.getenv("GHOST_LOCAL_MODEL", "qwen2.5-coder:latest")
    
    payload = {
        "model": model,
        "system": SYSTEM_PROMPT,
        "prompt": prompt,
        "format": "json",
        "stream": False,
        "options": {"temperature": 0.1},
    }

    async with httpx.AsyncClient(timeout=8.0) as client:
        resp = await client.post(f"{ollama_host}/api/generate", json=payload)
        if resp.status_code == 200:
            content = resp.json().get("response", "")
            parsed = _clean_json_output(content)
            if parsed:
                return LLMDiagnosis(
                    diagnosis=parsed.get("diagnosis", "Local diagnosis completed."),
                    suggested_fix=parsed.get("suggested_fix", "echo 'Check error output'"),
                    explanation=parsed.get("explanation", "Diagnosed via local Ollama fallback."),
                )
    return None


async def get_hybrid_diagnosis(
    command: str,
    exit_code: int,
    context: str,
    workspace: str,
    historical_notes: Optional[str] = None,
) -> Tuple[Optional[LLMDiagnosis], str]:
    """
    Attempts primary Cloud LLM (Groq / OpenRouter), then fails over to local Ollama.
    Returns (Diagnosis, EngineSource).
    """
    user_prompt = f"""Workspace Directory: {workspace}
Failed Command: {command}
Exit Code: {exit_code}
Captured Console Output Context:
{context}
"""
    if historical_notes:
        user_prompt += f"\nPrevious Workspace History:\n{historical_notes}\n"

    cloud_key = (
        os.getenv("GROQ_API_KEY")
        or os.getenv("OPENROUTER_API_KEY")
        or os.getenv("OPENAI_API_KEY")
    )

    # 1. Primary: Cloud LLM
    if cloud_key:
        try:
            res = await _query_groq_or_openrouter(user_prompt, cloud_key)
            if res:
                return res, "groq" if cloud_key.startswith("gsk_") or os.getenv("GROQ_API_KEY") else "openrouter"
        except Exception as e:
            print(f"[Ghost Cloud Engine Offline / Failed]: {e}. Attempting Ollama fallback...")

    # 2. Fallback: Local Ollama
    try:
        ollama_res = await _query_local_ollama(user_prompt)
        if ollama_res:
            return ollama_res, "ollama"
    except Exception as e:
        print(f"[Ghost Local Ollama Unavailable]: {e}")

    return None, "none"