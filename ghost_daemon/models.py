from typing import Optional
from pydantic import BaseModel, Field


class DiagnoseRequest(BaseModel):
    command: str = Field(description="The command that failed")
    exit_code: int = Field(description="Exit status code of the failed command")
    output_context: str = Field(description="Captured stderr/stdout console lines")
    cwd: Optional[str] = Field(default=".", description="Current working directory")
    project_type: Optional[str] = Field(default="unknown", description="Detected stack type")


class DiagnoseResponse(BaseModel):
    diagnosis: str = Field(description="Brief 1-sentence breakdown of why it failed")
    suggested_fix: str = Field(description="The exact executable CLI command fix")
    explanation: str = Field(default="", description="Detailed plain language explanation")
    confidence: float = Field(default=0.9, description="Confidence score 0.0-1.0")
    source: str = Field(default="stub", description="'fast_path' or 'llm' or 'stub'")