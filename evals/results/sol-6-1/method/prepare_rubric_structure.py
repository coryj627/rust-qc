#!/usr/bin/env python3
"""Encode the frozen authors' criterion IDs, maxima and allowed point increments."""

import hashlib
import json
from pathlib import Path


def entries(ids, maximum, increment=None):
    return {key: {"max": maximum, "increment": maximum if increment is None else increment}
            for key in ids.split()}


def main():
    structures = {
        "decom": {**entries("F1 F2 F3 F4", 1.5, 0.5),
                  **entries("R1 R2 R3 R5 A1 D1 D2", 0.5, 0.25),
                  **entries("R4", 1, 0.5), **entries("A2 U1 U2", 0.5)},
        "heapless": entries("B1 B2 B3 B4 B5 B6 B7 R1 R2 C1 D1 S1", 1),
        "wikilink": {**entries("B1", 2), **entries("B2 B3 B4 R1 R2 C1 C2 D1 S1 S2", 1)},
        "just": {**entries("B1 B2", 3, 0.5), **entries("C R", 2, 0.5),
                 **entries("D S", 1, 0.5)},
        "parser": entries("F1 F2 F3 F4 F5 F6 F7 F8 E1 E2 E3 E4 A1 A2 A3 A4 "
                          "T1 T2 T3 T4 D1 D2 S1 S2", 0.5),
        "library": entries("F1a F1b F2a F2b F3a F3b F4a F4b E1 E2a E2b E3 "
                           "A1 A2 A3a A3b T1 T2 T3 D1 D2 D3 U1 U2", 0.5),
    }
    root = Path(__file__).resolve().parent.parent
    result = {}
    for case, criteria in structures.items():
        if sum(item["max"] for item in criteria.values()) != 12:
            raise ValueError(f"Incorrect rubric total for {case}")
        rubric = root / "stage5/cases" / case / "rubric.md"
        result[case] = {"rubric_sha256": hashlib.sha256(rubric.read_bytes()).hexdigest(),
                        "quality_criteria": criteria,
                        "provenance": "Operator transcription of frozen rubric, before grading"}
    (root / "stage5/rubric-structure.json").write_text(json.dumps(result, indent=2) + "\n")
    print("Encoded six unchanged rubrics; all maxima sum to 12")


if __name__ == "__main__":
    main()
