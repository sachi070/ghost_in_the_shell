import sqlite3
import os
import subprocess
from typing import List, Dict, Any, Optional

DB_PATH = "ghost_session.db"


def resolve_workspace(cwd: Optional[str] = None) -> str:
    target_dir = cwd if cwd and os.path.exists(cwd) else os.getcwd()
    try:
        git_root = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=target_dir,
            stderr=subprocess.DEVNULL,
            text=True
        ).strip()
        if git_root:
            return git_root
    except Exception:
        pass
    return os.path.abspath(target_dir)


def init_db():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS interceptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            workspace TEXT NOT NULL,
            command TEXT NOT NULL,
            exit_code INTEGER NOT NULL,
            diagnosis TEXT,
            suggested_fix TEXT,
            engine_source TEXT DEFAULT 'unknown',
            was_accepted INTEGER DEFAULT 0
        )
    """)
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_workspace ON interceptions (workspace)")
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_command ON interceptions (command)")
    conn.commit()
    conn.close()


def log_interception(
    command: str,
    exit_code: int,
    diagnosis: str,
    suggested_fix: str,
    workspace: str,
    engine_source: str = "llm"
) -> int:
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute(
        """
        INSERT INTO interceptions (workspace, command, exit_code, diagnosis, suggested_fix, engine_source)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        (workspace, command, exit_code, diagnosis, suggested_fix, engine_source),
    )
    inserted_id = cursor.lastrowid
    conn.commit()
    conn.close()
    return inserted_id


def get_command_recurrence(command: str, exit_code: int, workspace: str) -> int:
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT COUNT(*) FROM interceptions
        WHERE workspace = ? AND command = ? AND exit_code = ?
        """,
        (workspace, command, exit_code),
    )
    count = cursor.fetchone()[0]
    conn.close()
    return count


def get_historical_context(command: str, workspace: str) -> Optional[str]:
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    base_cmd = command.strip().split()[0] if command.strip() else ""
    cursor.execute(
        """
        SELECT diagnosis, suggested_fix FROM interceptions
        WHERE workspace = ? AND command LIKE ?
        ORDER BY id DESC LIMIT 2
        """,
        (workspace, f"{base_cmd}%"),
    )
    rows = cursor.fetchall()
    conn.close()
    
    if not rows:
        return None
    
    history_snippets = [f"- Past Failure: {r[0]} | Past Fix: {r[1]}" for r in rows]
    return "\n".join(history_snippets)


def get_recent_interceptions(limit: int = 10, workspace: Optional[str] = None) -> List[Dict[str, Any]]:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    if workspace:
        cursor.execute(
            """
            SELECT id, timestamp, workspace, command, exit_code, diagnosis, suggested_fix, engine_source
            FROM interceptions
            WHERE workspace = ?
            ORDER BY id DESC
            LIMIT ?
            """,
            (workspace, limit),
        )
    else:
        cursor.execute(
            """
            SELECT id, timestamp, workspace, command, exit_code, diagnosis, suggested_fix, engine_source
            FROM interceptions
            ORDER BY id DESC
            LIMIT ?
            """,
            (limit,),
        )
    rows = cursor.fetchall()
    conn.close()
    return [dict(row) for row in rows]


def search_interceptions(query: str, limit: int = 10) -> List[Dict[str, Any]]:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    search_term = f"%{query}%"
    cursor.execute(
        """
        SELECT id, timestamp, workspace, command, exit_code, diagnosis, suggested_fix, engine_source
        FROM interceptions
        WHERE command LIKE ? OR diagnosis LIKE ? OR suggested_fix LIKE ?
        ORDER BY id DESC
        LIMIT ?
        """,
        (search_term, search_term, search_term, limit),
    )
    rows = cursor.fetchall()
    conn.close()
    return [dict(row) for row in rows]


def get_doctor_analytics() -> Dict[str, Any]:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    cursor.execute("SELECT COUNT(*) as total FROM interceptions")
    total_failures = cursor.fetchone()["total"]

    cursor.execute(
        """
        SELECT command, COUNT(*) as failure_count
        FROM interceptions
        GROUP BY command
        ORDER BY failure_count DESC
        LIMIT 5
        """
    )
    top_failing_commands = [dict(row) for row in cursor.fetchall()]

    cursor.execute(
        """
        SELECT workspace, COUNT(*) as failure_count
        FROM interceptions
        GROUP BY workspace
        ORDER BY failure_count DESC
        LIMIT 5
        """
    )
    workspace_breakdown = [dict(row) for row in cursor.fetchall()]

    cursor.execute(
        """
        SELECT engine_source, COUNT(*) as count
        FROM interceptions
        GROUP BY engine_source
        """
    )
    engine_sources = {row["engine_source"]: row["count"] for row in cursor.fetchall()}

    conn.close()
    return {
        "total_failures": total_failures,
        "top_failing_commands": top_failing_commands,
        "workspace_breakdown": workspace_breakdown,
        "engine_sources": engine_sources,
    }