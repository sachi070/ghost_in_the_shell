from typing import Optional
from sqlmodel import SQLModel, Field, create_engine, Session
from datetime import datetime


class AuditLog(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    timestamp: datetime = Field(default_factory=datetime.utcnow)
    command: str
    exit_code: int
    diagnosis: str
    suggested_fix: str
    accepted: Optional[bool] = Field(default=None)


DATABASE_URL = "sqlite:///./ghost_session.db"
engine = create_engine(DATABASE_URL, echo=False)


def init_db():
    SQLModel.metadata.create_all(engine)


def log_interception(command: str, exit_code: int, diagnosis: str, suggested_fix: str) -> int:
    with Session(engine) as session:
        log_entry = AuditLog(
            command=command,
            exit_code=exit_code,
            diagnosis=diagnosis,
            suggested_fix=suggested_fix,
        )
        session.add(log_entry)
        session.commit()
        session.refresh(log_entry)
        return log_entry.id