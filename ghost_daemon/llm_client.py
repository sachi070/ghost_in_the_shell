import json
import os
from typing import Optional
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


async def get_ai_diagnosis(
    command: str, exit_code: int, context: str, api_key: Optional[str] = None
) -> Optional[LLMDiagnosis]:
    # Prioritize GROQ_API_KEY, with fallback to OPENROUTER_API_KEY or OPENAI_API_KEY
    key = (
        api_key
        or os.getenv("GROQ_API_KEY")
        or os.getenv("OPENROUTER_API_KEY")
        or os.getenv("OPENAI_API_KEY")
    )
    if not key:
        return None

    # Determine endpoint and default model based on key type
    if os.getenv("GROQ_API_KEY") or key.startswith("gsk_"):
        endpoint = "https://api.groq.com/openai/v1/chat/completions"
        default_model = "llama-3.3-70b-versatile"
    else:
        endpoint = "https://openrouter.ai/api/v1/chat/completions"
        default_model = "google/gemini-2.5-flash"

    model = os.getenv("GHOST_LLM_MODEL", default_model)

    user_prompt = f"""Failed Command: {command}
Exit Code: {exit_code}
Captured Console Output Context:
{context}
"""

    headers = {
        "Authorization": f"Bearer {key}",
        "Content-Type": "application/json",
    }

    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_prompt},
        ],
        "response_format": {"type": "json_object"},
        "temperature": 0.1,
    }

    async with httpx.AsyncClient(timeout=10.0) as client:
        try:
            resp = await client.post(
                endpoint,
                headers=headers,
                json=payload,
            )
            if resp.status_code == 200:
                data = resp.json()
                content = data["choices"][0]["message"]["content"]
                parsed = json.loads(content)
                return LLMDiagnosis(
                    diagnosis=parsed.get("diagnosis", "Command failed execution."),
                    suggested_fix=parsed.get("suggested_fix", "echo 'No fix available'"),
                    explanation=parsed.get("explanation", ""),
                )
            else:
                print(f"[Ghost LLM HTTP Error {resp.status_code}]: {resp.text}")
        except Exception as e:
            print(f"[Ghost LLM Error]: {e}")
            return None
    return None