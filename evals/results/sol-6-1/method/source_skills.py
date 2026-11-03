#!/usr/bin/env python3
"""Enforce the sole-skill boundary for source trees as well as the skill catalog."""

import os
from pathlib import Path


def foreign_skills(workspace, allowed=None):
    expected = workspace / allowed / "SKILL.md" if allowed else None
    found = []
    for directory, folders, files in os.walk(workspace, followlinks=False):
        parent = Path(directory)
        folders[:] = [name for name in folders if name != "target"
                      and not (parent / name).is_symlink()]
        for name in files:
            path = parent / name
            if name.lower() == "skill.md" and (path != expected or path.is_symlink()):
                found.append(str(path.relative_to(workspace)))
    return sorted(found)


def require_sole_skill(workspace, allowed=None):
    found = foreign_skills(workspace, allowed)
    if found:
        raise ValueError(f"Other skill files are readable in the workspace: {found}")
