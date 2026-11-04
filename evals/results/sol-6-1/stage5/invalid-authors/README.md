# Invalidated author output

The original `author-just` workspace contained the upstream repository's own
`skills/just/SKILL.md`, in both the baseline and fixed source trees. Although it
was absent from the model's available-skills catalog, it was readable. The user's
sole-skill requirement therefore invalidates this author and its case bundle.

No coding treatment received this bundle. `author-just-v2` authored a new case
from pristine source with that directory excluded and no prior author output.
The valid bundle is `../cases/just/`; all six valid cases were committed at
`08dc878` before the first treatment. The invalid bundle is retained for audit
history and is excluded from grading and recommendations.

See `../../method/source-skill-isolation-review.json` for every affected run.
