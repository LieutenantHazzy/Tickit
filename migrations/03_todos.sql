CREATE TABLE todos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL,
  list_id INTEGER,
  title TEXT NOT NULL,
  description_md TEXT,
  done INTEGER NOT NULL DEFAULT 0,
  due_date TEXT,
  due_time TEXT,
  remind_at_utc TEXT,
  reminded_at TEXT,
  priority INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  FOREIGN KEY (list_id) REFERENCES lists(id) ON DELETE SET NULL
);
CREATE INDEX idx_todos_user ON todos(user_id);
CREATE INDEX idx_todos_due ON todos(user_id, due_date, due_time);
CREATE INDEX idx_todos_remind ON todos(remind_at_utc);
