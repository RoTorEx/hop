# Security Boundaries

## Default rules

- Do not commit secrets.
- Do not log raw secrets, tokens, private prompts, or sensitive payloads.
- Treat network access as an explicit project boundary.
- Treat local-first constraints as product truth.
- Manifest, environment, deployment, and auth configuration are child project truth.

## Parent vs child

The kernel defines safety principles.
Child projects define concrete boundaries.

Local overrides may strengthen boundaries.
Local overrides must not silently weaken boundaries.

## Agent behavior

Before changing security, network, storage, logging, or deployment boundaries:

1. read local overrides;
2. read relevant contracts/docs;
3. explain the boundary change;
4. ask for explicit approval if weakening anything.
