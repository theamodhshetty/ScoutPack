# Risk Hints

ScoutPack risk hints are lightweight inspection prompts.

They are not static analysis, security scanning, dataflow analysis, or test results.

## What Risk Hints Mean

Risk hints mean:

```txt
This selected code area may deserve extra inspection for this task.
```

Risk hints do not mean:

```txt
ScoutPack proved this defect exists.
```

## How They Are Generated

Current risk hints use deterministic rules over:

- task terms
- selected file paths
- selected snippets
- indexed symbols
- indexed source ranges

ScoutPack only emits a source-backed hint when selected context contains matching evidence. It does not parse runtime behavior or execute project code.

## Current Rule Families

| Rule family | Trigger | Example hint | Confidence |
| --- | --- | --- | --- |
| Redirect/login | task mentions login/auth/redirect and selected code contains redirect/login evidence | redirect loop around post-login destination or auth guard | medium |
| Session boundary | task mentions login/auth and selected code contains session/auth evidence | SSR/client mismatch around session state | low |
| Secrets/env | task mentions env/secret | secret files are intentionally skipped; verify env names manually | high |
| Tests | task mentions tests | test commands are indexed but not executed | medium |

## Confidence

Confidence is conservative:

- `high`: behavior follows from ScoutPack policy, for example secret files skipped
- `medium`: common issue pattern with source evidence, but still needs review
- `low`: broad inspection hint that may be irrelevant

## Limitations

- no taint tracking
- no control-flow/dataflow analysis
- no runtime validation
- no framework-specific proof
- no false-positive rate claim
- no security guarantee

Risk hints can echo task wording when task wording already names a risk. That is expected. The value is linking that inspection point to selected files and ranges, not discovering every possible defect.

## Maintainer Rule

Do not add impressive-sounding risk hints unless they are:

- deterministic
- source-backed when possible
- documented here
- tested
- worded as inspection guidance, not proof
