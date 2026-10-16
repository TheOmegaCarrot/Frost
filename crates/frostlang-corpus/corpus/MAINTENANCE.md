# Maintaining the corpus

The corpus pins what Frost should do, so it changes only when that does, or
when a script itself was wrong. The format is described in `README.md`.

## When the language changes

- **Reject to accept.** A change that intentionally makes a rejected script
  acceptable moves the script, changing only its location and its
  expectation.
- **Accept to reject.** A change that intentionally makes an accepted script
  rejected deletes it. A fresh `compile-error` script for the new rule is
  welcome, written new rather than moved.
- **New features.** A new core-language feature adds scripts that really
  exercise it: simple, subtle, and in combination with other features.
  Writing them also surfaces corner cases not yet considered.
- **Backlog work.** Work on any Backlog task updates the scripts it
  intentionally affects, in the same change.

## When behavior is wrong

- **Never assert wrong behavior.** A changed result is never a reason to
  re-pin an expectation to match: it is either a bug or an intentional change
  handled by the rules above.
- **Known issues.** A bug found in the corpus gets its own Backlog task. The
  script showing it keeps the correct expectation and carries
  `known_issue: '<task ID>'`.
- **Accepted regressions.** A regression accepted for now is a known issue
  with `regression: true` as well.
- **Fixes.** A bug fix removes that bug's `known_issue` markers in the same
  change; its task cannot close while any marker names it. The run fails on a
  stale marker.
- **Bug fixes earn a script.** Every core-language bug fix adds a script for
  it, even when a Rust test also covers it.

## Editing scripts

- **Don't dodge failures.** A script's body changes only when the script
  itself was wrong, never to dodge a failure.
- **Loosen, don't re-pin.** An error match broken by a rewording was too
  tight: loosen it to the part that is stable, rather than re-pinning it.
- **One concern per script.** Its file name says what it covers, and a brief
  prose comment says what it checks and, for a subtle case, why it is subtle.
- **Stay in scope.** Scripts cover the core language only: the domain of the
  compiler and the VM, plus the globals `README.md` names as in scope. They
  are deterministic: no clock, randomness,
  filesystem, or environment dependence.
