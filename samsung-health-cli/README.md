# shealth — Samsung Health from the command line

Query your Samsung Health data (steps, sleep, heart rate, workouts, weight,
stress, SpO₂) in the terminal, as text tables, JSON, or CSV. It has no
dependencies beyond Python 3.9+, uses no API keys, and nothing leaves your
machine.

## How it gets your data (read this first)

Samsung Health has **no public cloud API**. Your data lives on the phone and
in Samsung's private cloud. The only official way out without writing an
Android app is the app's built-in export, so that is what this tool reads:

1. On the phone, open **Samsung Health → ☰ / Settings → Download personal data → Download**.
2. The export appears in `Internal storage/Download/Samsung Health/samsunghealth_<you>_<timestamp>/`.
   It's a folder of CSV files, one for each data type.
3. Copy that folder, or a zip of it, to your computer and run `shealth import` on it.

This means the tool is **read-only** and works from a **snapshot**: to see
new data, export again and re-import. Re-importing replaces the old snapshot.

## Install

```bash
cd samsung-health-cli
pip install .            # adds the `shealth` command
# or run it without installing:
python3 -m shealth --help
```

## Use

```bash
shealth demo                       # try it on fake data first
shealth import ~/Downloads/samsunghealth_me_20261110120000   # folder or .zip

shealth summary                    # last 7 days: steps, sleep, HR, stress, workouts
shealth steps -d 30                # daily steps with bar chart
shealth sleep                      # sessions, score, deep/REM/light/awake minutes
shealth heart                      # daily low / avg / max bpm
shealth workouts -d 90
shealth weight -d 365
shealth stress
shealth spo2
shealth steps --from 2026-01-01 --to 2026-03-31 --csv > q1.csv
```

Date windows **end on the day you exported**, not today, so an old export
still shows data. Override this with `--to`.

### Everything else: `tables` and `sql`

Every CSV in the export is imported as its own SQLite table, so data types
without a dedicated command (water, food, floors, blood pressure, etc.) are
still available:

```bash
shealth tables                      # list all tables and row counts
shealth tables water_intake         # columns + sample rows
shealth sql "SELECT sh_date(start_time, time_offset) AS day, SUM(amount) AS ml
             FROM water_intake GROUP BY day ORDER BY day DESC LIMIT 7"
```

`sh_local(start_time, time_offset)` and `sh_date(...)` convert Samsung's UTC
timestamps to your local time. `sql` opens the database read-only.

### For scripts and AI agents

Every command accepts `--json`, so you can pipe the output into `jq`, a
notebook, or an LLM agent:

```bash
shealth summary --json | jq '.averages'
```

## Notes and known limits

* **Steps**: by default the tool uses Samsung's merged phone + watch total
  (`step_daily_trend`, `source_type = -2`). If that table is missing, it takes
  the highest-reporting device for each day rather than adding phone and watch
  together, which would double-count.
* **Sleep** is attributed to the date you woke up.
* **Heart "low"** is the 5th percentile of that day's readings. It's a robust
  resting-HR estimate, **not** Samsung's own resting-HR number.
* **Exercise types**: the common codes are mapped to names. Unknown ones
  show as `type <code>`.
* Samsung changes column names between app versions. The importer strips
  prefixes and looks for alternative names, but if a command says a data type
  is missing when you know you have it, run `shealth tables <name>` to see the
  real columns.
* The database lives at `~/.shealth/health.db`. Override it with `--db` or `$SHEALTH_DB`.
  **Don't commit your export or database to git.** `.gitignore` here blocks
  `*.zip`, `*.db` and `samsunghealth_*/`.

## Tests

```bash
python3 -m unittest discover -s tests -v
```
