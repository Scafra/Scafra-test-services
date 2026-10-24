# Scafra Petstore test service

A complete in-memory Petstore API used to exercise Scafra from a separate
repository. Scafra dependencies use the GitHub `main` branch, with the resolved
commit recorded in `Cargo.lock`. Run `cargo update -p scafra -p scafra-bootui`
to fetch the latest commit from `main`.

When developing against a sibling Scafra checkout at `../ember`, use the
helper to build, run, or test against those local crates:

```bash
./scripts/cargo-local-scafra build
./scripts/cargo-local-scafra test
```

Set `SCAFRA_CHECKOUT` when the checkout is elsewhere.

## Run

```bash
cargo run
```

Use the development profile on port `8081`:

```bash
SCAFRA_PROFILE=dev SCAFRA_DEV_RELOAD=false ./scripts/cargo-local-scafra run
```

The base settings are in `src/resources/application.yml`;
`src/resources/application-dev.yml` is merged when `SCAFRA_PROFILE=dev` is set.
Environment variables such as
`SCAFRA_SERVER_PORT=9090` override both files.

Application code is organized below `src/main/`, with controllers, services,
jobs, and models in their respective subdirectories.

The service also contains a scheduler example in
`src/main/jobs/pet_maintenance.rs`. It logs a maintenance heartbeat every 30
seconds by default, or every 5 seconds with `SCAFRA_PROFILE=dev`.
The job uses Scafra's structured `info!` macro and `#[logger]` instrumentation.

Scheduler configuration is profile-aware:

```yaml
scheduler:
  enabled: true
  tasks:
    pet-maintenance:
      enabled: true
      interval_ms: 30000
```

Set `scheduler.enabled` to `false`, or disable the individual task, to stop
the scheduled job.

The service listens on `127.0.0.1:8090` and exposes:

```text
GET    /health
GET    /live
GET    /ready
GET    /info
GET    /actuator/health
GET    /actuator/health/liveness
GET    /actuator/health/readiness
GET    /actuator/info
GET    /pet
GET    /pet/{id}
GET    /pet/findByStatus?status=available
GET    /pet/findByTags?tags=cat&tags=friendly
POST   /pet
PUT    /pet/{id}
DELETE /pet/{id}
```

## Live Scafra feature demo

Run the development profile to see the framework features in action:

```bash
SCAFRA_PROFILE=dev SCAFRA_DEV_RELOAD=false ./scripts/cargo-local-scafra run
```

The service listens on `127.0.0.1:8081` in this profile. Open
<http://127.0.0.1:18092/bootui> for the BootUI dashboard and the effective
configuration. Actuator health, readiness, liveness, info, and metrics are
available at `/health`, `/ready`, `/live`, `/info`, and `/metrics`.
Readiness checks that the PetStore bean was created during startup.
The health-check API is process-wide, so this sample reports whether any
PetStore instance is active in the process; it does not distinguish between
multiple application graphs in one process.
The current `/metrics` response is Scafra's fixed `scafra_up 1` placeholder;
it does not report Petstore request or business metrics yet (tracked by Scafra
issue #35).

The dev profile enables HTTP Basic authentication for the Pet API. Use
`demo:scafra-local`, for example:

```bash
curl -u demo:scafra-local http://127.0.0.1:8081/pet
```

The `pet-maintenance` scheduler logs a heartbeat every five seconds in this
profile. The base profile uses the same visible features with authentication
disabled and a 30-second scheduler interval.

Data is stored in memory and is reset whenever the service restarts.

## Example

```bash
curl -X POST http://127.0.0.1:8090/pet \
  -H 'content-type: application/json' \
  -d '{"name":"Milo","photoUrls":[],"tags":["cat"],"status":"available"}'

curl http://127.0.0.1:8090/pet/1
```

The HTTP contract is covered by the integration tests in `tests/http.rs`.
