# Governance

DumbMonit is a small open source project with a benevolent-dictator model: one
maintainer has the final say, and everyone is welcome to propose, discuss and
contribute.

## Roles

- **Maintainer.** Reviews and merges pull requests, triages issues, cuts
  releases, handles security reports (see [SECURITY.md](SECURITY.md)) and owns
  the repository, the container registry and the documentation hosting.
  Today this is [@laupernoe](https://github.com/laupernoe).
- **Contributors.** Anyone who opens an issue, sends a pull request, improves
  the documentation or adds a device profile or integration. See
  [CONTRIBUTING.md](CONTRIBUTING.md).
- **Translators.** Translate the web UI and the documentation by pull
  request (see [CONTRIBUTING.md](CONTRIBUTING.md#translating-dumbmonit)).

## How decisions are made

- Small changes: a pull request, reviewed by the maintainer.
- Larger changes (new component, API change, new dependency, anything touching
  authentication or secrets): open an issue first and discuss it there. The
  maintainer decides after the discussion and explains the decision in the
  issue.
- The project follows its [Code of Conduct](CODE_OF_CONDUCT.md).
- Direction is visible on the public
  [roadmap board](https://github.com/users/laupernoe/projects/2).

## Becoming a maintainer

There is no fixed ladder. A contributor who has sent several substantial,
well-tested pull requests, reviews others' work constructively, follows the
security and licence rules of the project, and wants to take on the
responsibility can ask the current maintainer, who grants repository access
and invites them to the maintainer role. A new maintainer's access is limited
at first to what the work needs.

## Continuity

The project currently has one maintainer. That is a risk, and we state it
plainly. Continuity requires at least one other trusted person who can, if the
maintainer is unavailable, merge and release, rotate credentials, and publish
security advisories: access to the GitHub repository and organisation settings,
the container registry (GHCR), the Read the Docs project
and the domain names. No second person has that access yet. Finding one is part
of the path described above; until then, the source code is Apache-2.0 licensed
and can be forked by anyone if the project stops.
