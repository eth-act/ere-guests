## Summary

<!-- What changes and why. -->

<!--
The PR title becomes the squash commit title, and Release Please picks the
next version from it. CI rejects titles without a Conventional Commit type.

The type sets the bump (examples from 0.18.0):

  feat:     0.19.0   minor
  fix:      0.18.1   patch
  perf:     0.18.1   patch
  revert:   0.18.1   patch
  chore:    none     same for build, ci, docs, refactor, style, test

Types with no bump don't trigger a release and stay out of the release
notes. Their changes still ship in the next release.

The largest unreleased bump on main wins: a fix: merged while a feat:
awaits release ships in that minor release.

To force a version, add this line to the squash commit body when merging:

  Release-As: <version>

If a merged PR got the wrong title, see "How can I fix release notes?":
https://github.com/googleapis/release-please#how-can-i-fix-release-notes
-->
