CREATE TABLE job_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  job_name TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('SUCCESS', 'PARTIAL_FAILURE', 'FAILED')),
  provider TEXT NOT NULL,
  summary JSONB NOT NULL DEFAULT '{}'::jsonb,
  started_at TIMESTAMPTZ NOT NULL,
  finished_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  error TEXT
);

CREATE INDEX idx_job_runs_name_finished_at ON job_runs(job_name, finished_at DESC);
