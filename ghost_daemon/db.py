import sqlite3
from typing import List, Dict, Any

DB_PATH = "ghost_session.db"


def init_db():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS interceptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            command TEXT NOT NULL,
            exit_code INTEGER NOT NULL,
            diagnosis TEXT,
            suggested_fix TEXT
        )
    """)
    conn.commit()
    conn.close()


def log_interception(command: str, exit_code: int, diagnosis: str, suggested_fix: str):
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute(
        """
        INSERT INTO interceptions (command, exit_code, diagnosis, suggested_fix)
        VALUES (?, ?, ?, ?)
    """,
        (command, exit_code, diagnosis, suggested_fix),
    )
    conn.commit()
    conn.close()


def get_recent_interceptions(limit: int = 10) -> List[Dict[str, Any]]:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT id, timestamp, command, exit_code, diagnosis, suggested_fix
        FROM interceptions
        ORDER BY id DESC
        LIMIT ?
    """,
        (limit,),
    )
    rows = cursor.fetchall()
    conn.close()
    return [dict(row) for row in rows]