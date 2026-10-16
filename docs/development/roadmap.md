# Roadmap

What is shipped, in progress and planned is tracked on the public project
board: [DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2).
Priorities and timing can change; the board is a living plan, not a promise.

- **Shipped changes** are in the
  [changelog](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  and in the notes of each
  [GitHub release](https://github.com/laupernoe/dumbmonit/releases).
- **Ideas and bugs** live in the
  [issue tracker](https://github.com/laupernoe/dumbmonit/issues). Open an issue
  before writing code for something large.
- **Security fixes** are announced in the release notes and as GitHub security
  advisories, with their CVE or GHSA identifier.

DumbMonit is before 1.0: expect the API and the data model to keep evolving until
a stable line is announced. See the
[contributing guide](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
and the
[governance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)
to take part.

## Direction for the next twelve months

These are intentions, not commitments, in rough order.

- **Towards 1.0.** Settle the HTTP API and the database schema so that a stable
  line can be announced; until then minor versions may still change them, with
  upgrade notes in the changelog.
- **More people with access.** A second administrator of the repository,
  registry, documentation hosting and domains (see the
  [governance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Supply chain.** Reproducible release builds
  ([current state](reproducible-builds.md)) and signed version tags.
- **arm64 images.** Server images are amd64 only for now; arm64 returns with
  native build runners.
- **Integrations and packs.** More device and service integrations and SNMP
  profiles, driven by contributions and requests.
- **Documentation.** Keep the nine languages of the interface and the
  documentation in step with the English source.

## What DumbMonit does not intend to do

- Become a log, trace or APM platform: it monitors devices and services.
- Offer a paid edition or a hosted service: it stays 100 % open source.
- Send telemetry about your instance anywhere.
- Target fleets of thousands of devices: it is built for homelabs and small
  teams, on one container.
