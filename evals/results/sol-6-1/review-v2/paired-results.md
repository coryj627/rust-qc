# Paired results

These tables retain both runs per arm and the absolute within-arm difference beside every project gap. Values are counts per report. Replicates follow the frozen schedule order. Exact sign-flip p-values are uncorrected. Non-defect hygiene was requested by the task and is not automatically harm.

Known-bug measures use only the eligible definitions. Verified-defect totals count report/finding memberships, not unique newly discovered bugs. See `runs.tsv` for run IDs and `summary.json` for per-run scores.

## Detailed reviews

### known_reproduced_found

Paired projects: 9. Skill/baseline totals: 34/32. Mean project gap: 0.111111; mean within-arm absolute difference: 0.555556; uncorrected p: 0.8125.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 1, 1 | 1, 1 | 0 | 0 | 0 |
| h2 | 0, 1 | 1, 1 | -0.5 | 1 | 0 |
| heapless | 2, 2 | 1, 1 | 1 | 0 | 0 |
| just | 5, 3 | 3, 4 | 0.5 | 2 | 1 |
| mlua | 2, 3 | 3, 2 | 0 | 1 | 1 |
| moka | 2, 2 | 2, 2 | 0 | 0 | 0 |
| quinn-proto | 2, 1 | 0, 1 | 1 | 1 | 1 |
| simd-json | 4, 3 | 5, 4 | -1 | 1 | 1 |
| zip | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: pulldown-cmark.

### known_reproduced_found_or_partial

Paired projects: 9. Skill/baseline totals: 36/32. Mean project gap: 0.222222; mean within-arm absolute difference: 0.555556; uncorrected p: 0.625.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 1, 1 | 1, 1 | 0 | 0 | 0 |
| h2 | 0, 1 | 1, 1 | -0.5 | 1 | 0 |
| heapless | 2, 2 | 1, 1 | 1 | 0 | 0 |
| just | 5, 3 | 3, 4 | 0.5 | 2 | 1 |
| mlua | 2, 3 | 3, 2 | 0 | 1 | 1 |
| moka | 2, 2 | 2, 2 | 0 | 0 | 0 |
| quinn-proto | 3, 2 | 0, 1 | 2 | 1 | 1 |
| simd-json | 4, 3 | 5, 4 | -1 | 1 | 1 |
| zip | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: pulldown-cmark.

### known_all_found

Paired projects: 9. Skill/baseline totals: 36/34. Mean project gap: 0.111111; mean within-arm absolute difference: 0.555556; uncorrected p: 0.84375.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 1, 1 | 1, 1 | 0 | 0 | 0 |
| h2 | 0, 1 | 1, 1 | -0.5 | 1 | 0 |
| heapless | 2, 2 | 1, 1 | 1 | 0 | 0 |
| just | 5, 3 | 3, 4 | 0.5 | 2 | 1 |
| mlua | 2, 3 | 3, 2 | 0 | 1 | 1 |
| moka | 2, 2 | 2, 2 | 0 | 0 | 0 |
| quinn-proto | 3, 2 | 1, 1 | 1.5 | 1 | 0 |
| simd-json | 4, 3 | 5, 4 | -1 | 1 | 1 |
| zip | 0, 0 | 1, 0 | -0.5 | 0 | 1 |

Excluded incomplete projects: pulldown-cmark.

### real_defects

Paired projects: 9. Skill/baseline totals: 232/206. Mean project gap: 1.44444; mean within-arm absolute difference: 2.66667; uncorrected p: 0.234375.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 15, 21 | 13, 11 | 6 | 6 | 2 |
| h2 | 5, 6 | 8, 6 | -1.5 | 1 | 2 |
| heapless | 14, 16 | 10, 13 | 3.5 | 2 | 3 |
| just | 11, 11 | 9, 12 | 0.5 | 0 | 3 |
| mlua | 10, 10 | 12, 8 | 0 | 0 | 4 |
| moka | 7, 5 | 5, 7 | 0 | 2 | 2 |
| quinn-proto | 16, 12 | 15, 14 | -0.5 | 4 | 1 |
| simd-json | 18, 14 | 18, 16 | -1 | 4 | 2 |
| zip | 19, 22 | 18, 11 | 6 | 3 | 7 |

Excluded incomplete projects: pulldown-cmark.

### real_high

Paired projects: 9. Skill/baseline totals: 94/67. Mean project gap: 1.5; mean within-arm absolute difference: 1.38889; uncorrected p: 0.0625.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 13, 15 | 8, 6 | 7 | 2 | 2 |
| h2 | 1, 0 | 0, 1 | 0 | 1 | 1 |
| heapless | 8, 9 | 4, 9 | 2 | 1 | 5 |
| just | 4, 3 | 2, 3 | 1 | 1 | 1 |
| mlua | 2, 2 | 3, 1 | 0 | 0 | 2 |
| moka | 3, 2 | 2, 3 | 0 | 1 | 1 |
| quinn-proto | 5, 4 | 3, 4 | 1 | 1 | 1 |
| simd-json | 4, 4 | 4, 5 | -0.5 | 0 | 1 |
| zip | 9, 6 | 5, 4 | 3 | 3 | 1 |

Excluded incomplete projects: pulldown-cmark.

### not_defect

Paired projects: 9. Skill/baseline totals: 90/33. Mean project gap: 3.16667; mean within-arm absolute difference: 0.611111; uncorrected p: 0.00390625.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 5, 5 | 1, 1 | 4 | 0 | 0 |
| h2 | 3, 5 | 2, 2 | 2 | 2 | 0 |
| heapless | 2, 5 | 2, 2 | 1.5 | 3 | 0 |
| just | 2, 2 | 1, 1 | 1 | 0 | 0 |
| mlua | 8, 8 | 2, 2 | 6 | 0 | 0 |
| moka | 7, 7 | 1, 2 | 5.5 | 0 | 1 |
| quinn-proto | 8, 5 | 4, 4 | 2.5 | 3 | 0 |
| simd-json | 4, 4 | 1, 1 | 3 | 0 | 0 |
| zip | 6, 4 | 2, 2 | 3 | 2 | 0 |

Excluded incomplete projects: pulldown-cmark.

### hygiene

Paired projects: 9. Skill/baseline totals: 86/28. Mean project gap: 3.22222; mean within-arm absolute difference: 0.555556; uncorrected p: 0.00390625.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 5, 5 | 1, 1 | 4 | 0 | 0 |
| h2 | 3, 5 | 2, 2 | 2 | 2 | 0 |
| heapless | 1, 5 | 1, 1 | 2 | 4 | 0 |
| just | 2, 2 | 1, 1 | 1 | 0 | 0 |
| mlua | 8, 8 | 2, 2 | 6 | 0 | 0 |
| moka | 7, 7 | 1, 1 | 6 | 0 | 0 |
| quinn-proto | 7, 5 | 4, 4 | 2 | 2 | 0 |
| simd-json | 3, 3 | 0, 0 | 3 | 0 | 0 |
| zip | 6, 4 | 2, 2 | 3 | 2 | 0 |

Excluded incomplete projects: pulldown-cmark.

### no_value

Paired projects: 9. Skill/baseline totals: 0/0. Mean project gap: 0; mean within-arm absolute difference: 0; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| heapless | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| mlua | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |
| quinn-proto | 0, 0 | 0, 0 | 0 | 0 | 0 |
| simd-json | 0, 0 | 0, 0 | 0 | 0 | 0 |
| zip | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: pulldown-cmark.

### other_nondefect

Paired projects: 9. Skill/baseline totals: 4/5. Mean project gap: -0.0555556; mean within-arm absolute difference: 0.166667; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| heapless | 1, 0 | 1, 1 | -0.5 | 1 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| mlua | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 1 | -0.5 | 0 | 1 |
| quinn-proto | 1, 0 | 0, 0 | 0.5 | 1 | 0 |
| simd-json | 1, 1 | 1, 1 | 0 | 0 | 0 |
| zip | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: pulldown-cmark.

### refuted

Paired projects: 9. Skill/baseline totals: 0/0. Mean project gap: 0; mean within-arm absolute difference: 0; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| heapless | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| mlua | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |
| quinn-proto | 0, 0 | 0, 0 | 0 | 0 | 0 |
| simd-json | 0, 0 | 0, 0 | 0 | 0 | 0 |
| zip | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: pulldown-cmark.

### unverified

Paired projects: 9. Skill/baseline totals: 2/0. Mean project gap: 0.111111; mean within-arm absolute difference: 0.111111; uncorrected p: 0.5.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| git2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| heapless | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 1 | 0, 0 | 0.5 | 1 | 0 |
| mlua | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |
| quinn-proto | 0, 0 | 0, 0 | 0 | 0 | 0 |
| simd-json | 1, 0 | 0, 0 | 0.5 | 1 | 0 |
| zip | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: pulldown-cmark.

## Short reviews

### known_reproduced_found

Paired projects: 3. Skill/baseline totals: 10/13. Mean project gap: -0.5; mean within-arm absolute difference: 0.5; uncorrected p: 0.5.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 2 | -1 | 0 | 2 |
| just | 3, 3 | 3, 4 | -0.5 | 0 | 1 |
| moka | 2, 2 | 2, 2 | 0 | 0 | 0 |

Excluded incomplete projects: none.

### known_reproduced_found_or_partial

Paired projects: 3. Skill/baseline totals: 10/13. Mean project gap: -0.5; mean within-arm absolute difference: 0.5; uncorrected p: 0.5.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 2 | -1 | 0 | 2 |
| just | 3, 3 | 3, 4 | -0.5 | 0 | 1 |
| moka | 2, 2 | 2, 2 | 0 | 0 | 0 |

Excluded incomplete projects: none.

### known_all_found

Paired projects: 3. Skill/baseline totals: 10/13. Mean project gap: -0.5; mean within-arm absolute difference: 0.5; uncorrected p: 0.5.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 2 | -1 | 0 | 2 |
| just | 3, 3 | 3, 4 | -0.5 | 0 | 1 |
| moka | 2, 2 | 2, 2 | 0 | 0 | 0 |

Excluded incomplete projects: none.

### real_defects

Paired projects: 3. Skill/baseline totals: 28/34. Mean project gap: -1; mean within-arm absolute difference: 2.33333; uncorrected p: 0.5.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 4, 3 | 4, 7 | -2 | 1 | 3 |
| just | 4, 12 | 7, 7 | 1 | 8 | 0 |
| moka | 2, 3 | 4, 5 | -2 | 1 | 1 |

Excluded incomplete projects: none.

### real_high

Paired projects: 3. Skill/baseline totals: 4/7. Mean project gap: -0.5; mean within-arm absolute difference: 0.166667; uncorrected p: 0.5.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 1, 1 | 1, 2 | -0.5 | 0 | 1 |
| moka | 1, 1 | 2, 2 | -1 | 0 | 0 |

Excluded incomplete projects: none.

### not_defect

Paired projects: 3. Skill/baseline totals: 23/6. Mean project gap: 2.83333; mean within-arm absolute difference: 0.166667; uncorrected p: 0.25.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 3, 3 | 1, 1 | 2 | 0 | 0 |
| just | 2, 2 | 1, 1 | 1 | 0 | 0 |
| moka | 6, 7 | 1, 1 | 5.5 | 1 | 0 |

Excluded incomplete projects: none.

### hygiene

Paired projects: 3. Skill/baseline totals: 23/6. Mean project gap: 2.83333; mean within-arm absolute difference: 0.166667; uncorrected p: 0.25.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 3, 3 | 1, 1 | 2 | 0 | 0 |
| just | 2, 2 | 1, 1 | 1 | 0 | 0 |
| moka | 6, 7 | 1, 1 | 5.5 | 1 | 0 |

Excluded incomplete projects: none.

### no_value

Paired projects: 3. Skill/baseline totals: 0/0. Mean project gap: 0; mean within-arm absolute difference: 0; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: none.

### other_nondefect

Paired projects: 3. Skill/baseline totals: 0/0. Mean project gap: 0; mean within-arm absolute difference: 0; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: none.

### refuted

Paired projects: 3. Skill/baseline totals: 0/0. Mean project gap: 0; mean within-arm absolute difference: 0; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: none.

### unverified

Paired projects: 3. Skill/baseline totals: 0/0. Mean project gap: 0; mean within-arm absolute difference: 0; uncorrected p: 1.

| Project | Skill runs | Baseline runs | Gap | Within skill | Within baseline |
|---|---:|---:|---:|---:|---:|
| h2 | 0, 0 | 0, 0 | 0 | 0 | 0 |
| just | 0, 0 | 0, 0 | 0 | 0 | 0 |
| moka | 0, 0 | 0, 0 | 0 | 0 | 0 |

Excluded incomplete projects: none.

