# 👻 Ghost in the Shell — Audit Report

Total Records: 2

| Timestamp | Command | Exit Code | Engine | Diagnosis | Fix |
|---|---|---|---|---|---|
| 2026-08-18 09:42:53 | `git checkout non_existent_branch_12345` | 128 | stub | Command 'git checkout non_existent_branch_12345' failed with exit code 128. | `ghost doctor` |
| 2026-08-18 09:42:14 | `cat definitely_missing_test.txt` | 1 | fast_path | Target file 'definitely_missing_test.txt' does not exist in the current directory. | `touch definitely_missing_test.txt` |
