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
shealth sleep                      # nights: stages, snoring, sleeping HR/SpO2 (see "Sleep in depth")
shealth heart                      # daily low / avg / max bpm
shealth workouts -d 90
shealth weight -d 365
shealth stress
shealth spo2
shealth steps --from 2026-01-01 --to 2026-03-31 --csv > q1.csv
```

Date windows **end on the day you exported**, not today, so an old export
still shows data. Override this with `--to`.

## Sleep in depth

```bash
shealth sleep                 # one row per night: stages, awakenings, snoring, sleeping HR, SpO2, breathing
shealth sleep --all           # + every raw field from the sleep session table
shealth night                 # the most recent night, in full
shealth night 2026-11-09      # a specific night (the date you woke up)
shealth night --json          # everything, machine-readable
shealth serve                 # sleep dashboard in your browser (local only)
```

`shealth night` shows:

* **Hypnogram**: Awake / REM / Light / Deep over the night, with snoring
  episodes marked underneath.
* **Stage totals and derived numbers**: actual time asleep, time to fall
  asleep, number of awakenings, minutes awake after first falling asleep,
  and REM periods.
* **Measurements taken during the night**: heart rate, blood oxygen
  (including how many readings were under 90%), respiratory rate, skin
  temperature, HRV and stress, each with min/avg/max and a sparkline.
* **Every field Samsung stored for that session**: sleep score,
  efficiency, mental/physical recovery, sleep cycles, latency, movement,
  and whatever else your app version records.
* **Every other sleep table in the export** that has rows for that night,
  even ones this tool has never heard of.

**How it finds data:** Samsung doesn't publish its export columns, and they
vary by app version and device. So instead of hard-coding names, the tool
*discovers* data. Any table with "sleep", "snor", "vascular" (Vascular Load)
or "apnea" in its name is linked to a night, either by `sleep_id` or by its time range. Heart rate, SpO₂, breathing,
skin temperature, HRV and stress readings are matched by table name and
clipped to the time you were in bed.

**What you get depends on your devices and settings.** Stages, SpO₂, skin
temperature and respiratory rate need a Galaxy Watch, or a Ring, that
measures them, with those features turned on. Snoring needs **Snore
detection** turned on in Samsung Health → Sleep. When something is missing,
`shealth night` tells you so rather than showing a blank. One more caveat:
the snoring, respiratory-rate and skin-temperature file names in
`shealth demo` are plausible guesses, because the real ones aren't publicly
documented. If your export names them differently but they contain
"sleep"/"snor" or the keywords above, they're still picked up. If not,
`shealth tables` shows what's there.

### The dashboard (`shealth serve`)

A local web page at `http://127.0.0.1:8765` showing nightly stage bars for the
last 14/30/90 nights. Click a night to see its hypnogram with snoring marks,
the vitals charts, and every recorded field. It works on desktop and phone
widths, in light and dark mode.

* The server runs on the Python standard library and the page loads no
  external scripts or fonts, so it works offline.
* It listens on 127.0.0.1 only, so other devices can't reach it. `--host
  0.0.0.0` makes it reachable from your phone on the same Wi-Fi, but then
  anyone on that network can also see your health data.

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
