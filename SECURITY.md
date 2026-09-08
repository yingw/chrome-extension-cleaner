# Security Policy

Chrome Extension Cleaner deletes files on the local machine, so we take
vulnerability reports seriously.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Use GitHub's private vulnerability reporting instead:

1. Go to the **Security** tab of this repository.
2. Click **Report a vulnerability**.
3. Describe the issue. Reports stay private until we publish a fix.

If private reporting is unavailable, e-mail the maintainer directly (see the
Git history / commit author for the contact) and mark the message
**confidential**.

## What to include

- Steps to reproduce, or a minimal proof of concept.
- The platform and Chrome version affected.
- What you expected vs. what happened — especially if the tool deleted or could
  delete something outside `<Profile>/Extensions/<id>/<version>_0`.

## Scope notes

This is an unsigned experimental project. Reports about the following are
welcome but expected behavior, not vulnerabilities:

- Gatekeeper / SmartScreen warnings (unsigned builds, by design).
- Deletion of folders the tool is documented to delete (see README →
  Safety and privacy).

## Supported versions

Only the latest released build is supported. Please upgrade before reporting.
