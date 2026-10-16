# Governance

DumbMonit is a small open source project with a benevolent-dictator model: one
lead maintainer has the final say, and everyone is welcome to propose, discuss
and contribute.

## Roles

- **Maintainer.** Reviews and merges pull requests, triages issues, cuts
  releases, handles security reports (see [SECURITY.md](SECURITY.md)) and owns
  the repository, the container registry and the documentation hosting.
  Today this is [@laupernoe](https://github.com/laupernoe), who also holds the
  administrator rights on the GitHub repository.
- **Collaborators.** Regular contributors who were given write access to the
  repository. They review and merge pull requests (never their own without a
  review or a green CI), triage issues, and take part in design discussions.
  Today this is [@Theroxat](https://github.com/Theroxat), who has contributed
  regularly since the project's early weeks. Collaborators do not have
  administrator rights: the branch rulesets, release tags, secrets and
  hosting accounts below stay with the lead maintainer.
- **Contributors.** Anyone who opens an issue, sends a pull request, improves
  the documentation or adds a device profile or integration. See
  [CONTRIBUTING.md](CONTRIBUTING.md).
- **Translators.** Translate the web UI and the documentation by pull
  request (see [CONTRIBUTING.md](CONTRIBUTING.md#translating-dumbmonit)).

## How decisions are made

- Small changes: a pull request, reviewed by the maintainer or a collaborator.
  `main` and `dev` are protected by rulesets: changes arrive by pull request
  with the required CI checks green.
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

Two people can now review and merge: the lead maintainer and one collaborator
with write access. That covers day-to-day work (creating and closing issues,
accepting changes) if the lead maintainer is unavailable.

What is **not** yet covered is administration. Only the lead maintainer can
change repository settings and rulesets, push `v*` release tags (reserved to
repository administrators), manage the repository secrets and the signing
identity of the release workflow, publish security advisories, and manage the
container registry (GHCR), the Read the Docs project and the domain names. If
the lead maintainer disappeared, a collaborator could keep merging fixes to
`dev` but could not publish a release or an advisory until an administrator
was added. Closing that gap means adding a second administrator of the
repository and organisation, and a second owner of the Read the Docs project
and of the domain names; it is the next step of the path described above.
Until then, the source code is Apache-2.0 licensed and can be forked by anyone
if the project stops.
