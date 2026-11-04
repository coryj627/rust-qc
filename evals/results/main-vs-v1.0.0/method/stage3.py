#!/usr/bin/env python3
"""Stage 3 for one project: audits, blind grading, disputes, verification and per-run rows.

Usage: stage3.py <step> <clone> <results> <project>
Steps, in order: audits, collect, grade, disputes, verify, rows, clean; or all.

Every step skips work whose output already exists, so a run interrupted by a VM restart resumes.
Private state (arm map, letters, neutral key ids, where false findings went) stays under
/srv/rqc/stage3/<project>, which no agent can read.
"""

import json
import pathlib
import random
import re
import shutil
import subprocess
import sys

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import AUDITS, CLOCK, NOTES, RUN
from redact import redact
from streams import summary, tool_calls

HERE = pathlib.Path(__file__).parent
GRADING = pathlib.Path("/srv/rqc/grading")
STATE = pathlib.Path("/srv/rqc/stage3")
RNG = random.SystemRandom()
SEVERITY = re.compile(r"^##\s+(Blockers?|Major|Minor|Notes?|High|Medium|Low)\b", re.IGNORECASE)


def stream(name):
    return AUDITS / "private" / "streams" / f"{name}.jsonl"


def finished(name):
    return stream(name).exists() and summary(stream(name))["subtype"] is not None


def launch(names, *flags):
    pending = [n for n in names if not finished(n)]
    RNG.shuffle(pending)
    if pending:
        subprocess.run(
            [sys.executable, "-I", str(HERE / "launch.py"), *pending, *flags], check=True
        )


def runs_of(project):
    rows = (AUDITS / "private" / "map.tsv").read_text().splitlines()
    return {r.split("\t")[0]: r.split("\t")[2] for r in rows if r.split("\t")[1] == project}


def json_lines(text):
    out = []
    for line in (text or "").splitlines():
        line = line.strip().strip("`")
        if line.startswith("{") and line.endswith("}"):
            out.append(json.loads(line))
    return out


def append_jsonl(path, rows):
    with path.open("a") as f:
        f.writelines(json.dumps(r, ensure_ascii=False) + "\n" for r in rows)


def audits(clone, results, project):
    if not runs_of(project):
        subprocess.run(
            [sys.executable, "-I", str(HERE / "prep_workspaces.py"), "runs", project, "2"],
            check=True,
        )
    launch(list(runs_of(project)))


def collect(clone, results, project):
    state = STATE / project
    state.mkdir(parents=True, exist_ok=True)
    letters_file = state / "letters.json"
    runs = runs_of(project)
    if not letters_file.exists():
        ids = list(runs)
        RNG.shuffle(ids)
        letters_file.write_text(json.dumps(dict(zip("ABCD", ids))))
    letters = json.loads(letters_file.read_text())
    (results / "reports" / project).mkdir(parents=True, exist_ok=True)
    (results / "redacted" / project).mkdir(parents=True, exist_ok=True)
    dropped_all = []
    for letter, run_id in letters.items():
        text = summary(stream(run_id))["result"] or ""
        short = text.replace(str(AUDITS / "projects" / run_id), "<workspace>")
        (results / "reports" / project / f"{runs[run_id]}-{run_id}.md").write_text(short)
        redacted, dropped = redact(text, run_id)
        (results / "redacted" / project / f"report-{letter}.md").write_text(redacted)
        dropped_all += [f"report-{letter}\t{reason}\t{line}\n" for reason, line in dropped]
    (results / "redacted" / project / "dropped-lines.txt").write_text("".join(dropped_all))


def grader_key(results, project, state):
    """Real reproduced entries and the decoys, shuffled under neutral ids. Saves the id map."""
    key = yaml.safe_load((results / "stage2" / "keys" / f"{project}.yaml").read_text())
    entries = [
        (b["id"], {k: b[k] for k in ("summary", "failure_rule", "cause_rule", "match_rule")})
        for b in key["bugs"]
        if "failure_rule" in b
    ]
    controls = json.loads((results / "stage2" / "controls" / f"{project}.json").read_text())
    for d in controls["decoy_bugs"]:
        # Decoys have no match_rule; their summary stands in, so they look like the real entries.
        fields = {"summary": d["summary"], "failure_rule": d["failure_rule"]}
        fields |= {"cause_rule": d["cause_rule"], "match_rule": d["summary"]}
        entries.append((d["id"], fields))
    RNG.shuffle(entries)
    idmap = {f"e{n}": real for n, (real, _) in enumerate(entries, 1)}
    (state / "idmap.json").write_text(json.dumps(idmap))
    return [{"id": f"e{n}", **fields} for n, (_, fields) in enumerate(entries, 1)]


def grade(clone, results, project):
    state = STATE / project
    if not (state / "idmap.json").exists():
        key = grader_key(results, project, state)
        for g in ("g1", "g2"):
            folder = GRADING / f"{g}-{project}"
            folder.mkdir(parents=True, exist_ok=True)
            for letter in "ABCD":
                shutil.copy(results / "redacted" / project / f"report-{letter}.md", folder)
            (folder / "key.yaml").write_text(yaml.safe_dump(key, sort_keys=False, width=110))
            brief = (HERE / "grader-brief.md").read_text()
            (folder / "BRIEF.md").write_text(brief.format(name=NOTES[project][0], folder=folder))
    launch([f"g1-{project}", f"g2-{project}"], "--root", str(GRADING), "--read-only")
    if (state / "grades-written").exists():
        return
    idmap = json.loads((state / "idmap.json").read_text())
    rows = []
    for n, g in ((1, "g1"), (2, "g2")):
        lines = json_lines(summary(stream(f"{g}-{project}"))["result"])
        if len(lines) != 4 * len(idmap):
            raise SystemExit(f"{g}-{project}: {len(lines)} lines, expected {4 * len(idmap)}")
        for line in lines:
            rows.append(
                {
                    "project": project,
                    "grader": n,
                    "bug": idmap[line["bug"]],
                    "report": line["report"],
                    "failure": line["failure"],
                    "cause": line.get("cause"),
                    "original": line["original"],
                    "ref": line.get("ref") or None,
                    "why": line.get("why", ""),
                }
            )
    append_jsonl(results / "grades.jsonl", rows)
    (state / "grades-written").write_text("")


def disputes(clone, results, project):
    state = STATE / project
    if (state / "disputes-written").exists():
        return
    grades = [json.loads(x) for x in (results / "grades.jsonl").read_text().splitlines()]
    mine = [g for g in grades if g["project"] == project]
    by = {(g["bug"], g["report"], g["grader"]): g for g in mine}
    back = {
        real: neutral for neutral, real in json.loads((state / "idmap.json").read_text()).items()
    }
    found = []
    for (bug, report, grader), g1 in sorted(by.items()):
        if grader != 1:
            continue
        g2 = by[(bug, report, 2)]
        for field in ("failure", "cause"):
            if g1[field] != g2[field]:
                found.append(
                    {
                        "n": len(found) + 1,
                        "bug": back[bug],
                        "report": report,
                        "field": field,
                        "grader1": {"score": g1[field], "ref": g1["ref"], "why": g1["why"]},
                        "grader2": {"score": g2[field], "ref": g2["ref"], "why": g2["why"]},
                    }
                )
    if found:
        folder = GRADING / f"d-{project}"
        folder.mkdir(parents=True, exist_ok=True)
        for name in [f"report-{x}.md" for x in "ABCD"] + ["key.yaml"]:
            shutil.copy(GRADING / f"g1-{project}" / name, folder)
        (folder / "disputes.jsonl").write_text("".join(json.dumps(d) + "\n" for d in found))
        brief = (HERE / "dispute-brief.md").read_text()
        (folder / "BRIEF.md").write_text(brief.format(name=NOTES[project][0], folder=folder))
        launch([f"d-{project}"], "--root", str(GRADING), "--read-only")
        decisions = {d["n"]: d for d in json_lines(summary(stream(f"d-{project}"))["result"])}
        if set(decisions) != {d["n"] for d in found}:
            raise SystemExit(f"d-{project}: decisions do not cover every dispute")
        idmap = json.loads((state / "idmap.json").read_text())
        append_jsonl(
            results / "disputes.jsonl",
            [
                {
                    "project": project,
                    "bug": idmap[d["bug"]],
                    "report": d["report"],
                    "field": d["field"],
                    "grader1": d["grader1"]["score"],
                    "grader2": d["grader2"]["score"],
                    "decision": decisions[d["n"]]["decision"],
                    "why": decisions[d["n"]].get("why", ""),
                }
                for d in found
            ],
        )
    (state / "disputes-written").write_text(str(len(found)))


def insert_false_findings(results, project, folder, state):
    """Put each false finding into a random report, before a random real finding heading."""
    controls = json.loads((results / "stage2" / "controls" / f"{project}.json").read_text())
    reports = {x: (results / "redacted" / project / f"report-{x}.md").read_text() for x in "ABCD"}
    placed = {}
    for ff in controls["false_findings"]:
        letter = RNG.choice("ABCD")
        lines = reports[letter].splitlines()
        section, spots = None, []
        for i, line in enumerate(lines):
            if line.startswith("## "):
                section = line
            elif line.startswith("**") and section and SEVERITY.match(section):
                spots.append(i)
        at = RNG.choice(spots) if spots else len(lines)
        lines[at:at] = [*ff["text"].strip().splitlines(), ""]
        reports[letter] = "\n".join(lines) + "\n"
        placed[ff["id"]] = letter
    (folder / "reports").mkdir(exist_ok=True)
    for letter, text in reports.items():
        (folder / "reports" / f"report-{letter}.md").write_text(text)
    (state / "false-findings.json").write_text(json.dumps(placed))


def loc_of(text):
    match = re.search(r"([\w./-]+\.rs):(\d+)", text)
    return (pathlib.PurePosixPath(match.group(1)).name, int(match.group(2))) if match else None


def verify(clone, results, project):
    state = STATE / project
    name = f"v-{project}"
    folder = AUDITS / "projects" / name
    if not folder.exists():
        source = AUDITS / "projects" / f"tmpl-{project}"
        subprocess.run(["cp", "-a", str(source), str(folder)], check=True)
        for script, text in (("run", RUN), ("clock", CLOCK)):
            (folder / script).write_text(text)
            (folder / script).chmod(0o755)
        insert_false_findings(results, project, folder, state)
        title, path, build = NOTES[project]
        brief = (HERE / "verifier-brief.md").read_text()
        (folder / "BRIEF.md").write_text(
            brief.format(name=title, folder=folder, path=path, build=build)
        )
    launch([name])
    if (state / "verifier-written").exists():
        return
    controls = json.loads((results / "stage2" / "controls" / f"{project}.json").read_text())
    ff_locs = {ff["id"]: loc_of(ff["text"]) for ff in controls["false_findings"]}
    rows = []
    for item in json_lines(summary(stream(name))["result"]):
        if "item" not in item:
            continue
        here = loc_of(str(item.get("loc", "")))
        control = next(
            (
                fid
                for fid, loc in ff_locs.items()
                if loc and here and loc[0] == here[0] and abs(loc[1] - here[1]) <= 5
            ),
            None,
        )
        rows.append(
            {
                "project": project,
                "item": item["item"],
                "loc": item.get("loc"),
                "reports": item.get("reports"),
                "status": item["status"],
                "severity": item.get("severity"),
                "note": item.get("note", ""),
                "control": control,
            }
        )
    append_jsonl(results / "verifier.jsonl", rows)
    (state / "verifier-written").write_text(str(len(rows)))


def sections(report):
    """Count finding headings per kind of section, and the defect findings' Cause and Evidence."""
    counts = {"defects": 0, "hygiene": 0, "unverified": 0, "cause": 0, "evidence": 0}
    kind, finding = None, None
    for line in report.splitlines():
        if line.startswith("## "):
            kind = (
                "defects"
                if SEVERITY.match(line)
                else "hygiene"
                if re.match(r"##\s+QA hygiene", line, re.IGNORECASE)
                else "unverified"
                if re.match(r"##\s+Unverified", line, re.IGNORECASE)
                else None
            )
            finding = None
        elif line.startswith("### ") and kind:
            counts[kind] += 1
            finding = {"cause": False, "evidence": False} if kind == "defects" else None
        elif finding is not None:
            if re.match(r"\s*(\*\*)?Cause(\*\*)?:", line) and not finding["cause"]:
                finding["cause"] = True
                counts["cause"] += 1
            if re.match(r"\s*(\*\*)?Evidence \((ran|read)\)", line) and not finding["evidence"]:
                finding["evidence"] = True
                counts["evidence"] += 1
    return counts


def without_tail(report):
    """The report without its verification section and closing summary."""
    out, skip = [], False
    for line in report.splitlines():
        if line.startswith("#"):
            skip = bool(re.match(r"^#{1,3}\s+(Verification|Closing summary)\b", line))
        if not skip:
            out.append(line)
    return "\n".join(out)


def rows(clone, results, project):
    state = STATE / project
    if (state / "rows-written").exists():
        return
    letters = {
        run: letter for letter, run in json.loads((state / "letters.json").read_text()).items()
    }
    runs = runs_of(project)
    agents = [
        line.split("\t") for line in (AUDITS / "private" / "agents.tsv").read_text().splitlines()
    ]
    exits = {a[0]: int(a[4]) for a in agents}
    run_rows, measure_rows = [], []
    for run_id, arm in sorted(runs.items()):
        s = summary(stream(run_id))
        report = s["result"] or ""
        log = AUDITS / "projects" / run_id / ".clock-log"
        minutes = log.read_text().split()[-1] if log.exists() and log.read_text().strip() else ""
        calls = list(tool_calls(stream(run_id)))
        kills = _kills(run_id)
        run_rows.append(
            [
                run_id,
                project,
                arm,
                letters[run_id],
                minutes,
                str(exits.get(run_id) == 124 or (minutes != "" and int(minutes) >= 45)).lower(),
                str(kills),
                "",
            ]
        )
        c = sections(report)
        run_calls = sum(
            1
            for n, d, ran in calls
            if n == "Bash" and ran and d.get("command", "").lstrip().startswith("./run")
        )
        measure_rows.append(
            [
                run_id,
                project,
                arm,
                len(report.split()),
                len(without_tail(report).split()),
                c["defects"],
                c["hygiene"],
                c["unverified"],
                c["cause"],
                c["evidence"],
                run_calls,
                s["input_tokens"],
                s["output_tokens"],
                s["cache_read_tokens"],
                s["cache_write_tokens"],
                s["reported_cost_usd"],
            ]
        )
    for name, header, table in (
        (
            "runs.tsv",
            "run\tproject\tarm\tletter\tminutes\thit_limit\tcommand_kills\tstdin_hang_minutes",
            run_rows,
        ),
        (
            "measures.tsv",
            "\t".join(MEASURES),
            measure_rows,
        ),
    ):
        path = results / name
        if not path.exists():
            path.write_text(header + "\n")
        have = {line.split("\t")[0] for line in path.read_text().splitlines()[1:]}
        with path.open("a") as f:
            f.writelines(
                "\t".join("" if v is None else str(v) for v in row) + "\n"
                for row in table
                if row[0] not in have
            )
    (state / "rows-written").write_text("")


def _kills(run_id):
    """Commands the sandbox killed for memory or time, from the tool results in the stream."""
    text = stream(run_id).read_text()
    return text.count("sb: container killed (exit 137)")


def clean(clone, results, project):
    for run_id in [*runs_of(project), f"v-{project}"]:
        shutil.rmtree(AUDITS / "projects" / run_id, ignore_errors=True)
        subprocess.run(
            ["docker", "volume", "rm", f"rqc-target-{run_id}"], capture_output=True, check=False
        )


MEASURES = [
    "run",
    "project",
    "arm",
    "report_words",
    "report_words_no_verification",
    "defects_listed",
    "hygiene_items",
    "unverified_items",
    "findings_with_cause",
    "findings_with_evidence",
    "run_calls",
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reported_cost_usd",
]
STEPS = [audits, collect, grade, disputes, verify, rows, clean]


def main():
    step, clone, results, project = sys.argv[1], *map(pathlib.Path, sys.argv[2:4]), sys.argv[4]
    chosen = STEPS if step == "all" else [s for s in STEPS if s.__name__ == step]
    if not chosen:
        raise SystemExit(__doc__)
    for fn in chosen:
        print(f"== {project} {fn.__name__}", flush=True)
        fn(clone, results, project)


if __name__ == "__main__":
    main()
