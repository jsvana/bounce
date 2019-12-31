# bounce

An attempt to create a more convenient IRC bouncer with specific focus on message replay.

## Proposed Log Storage
```
  <server:hostport>/
    <channel>/
      METADATA
      <day_log>
      [...day_log]
```

A single copy of every server's logs will be stored.

>[!WARNING]
>Note that this means encryption will be harder to do properly.

```
      Read
      ───▶               ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
                                                Bounce Server
      Write              │                                                         │     foo.bar:1234 connection
      ════▶                                                                  MPSC      ────────────────────────────
                         │                           ┌───────────────┐      queue  │
         ╔═══════════════════════════════════════════│ foo.bar:1234  │◀═══════════════════════════╗
         ║               │                           └───────────────┘             │              ║
         ▼                                                                                        ║
┌────────────────┐       │       ┌────────────┐                                    │        ┌──────────┐
│     Actual     │               │    Ping    │      ┌───────────┐                          │ User 1's │
│  foo.bar:1234  │───────┼──────▶│(keepalive)?│─no──▶│    Log    │─────────────────┼───────▶│IRC client│
│     server     │               └────────────┘      └───────────┘                          └──────────┘
└────────────────┘       │              ║                                          │
         ▲                              ║
         ║               │              ║                                          │
         ╚═════════════════════════yes══╝
                         │                                                         │

                         │                                                         │

         ╔═══════════════╬═════════yes══╗                                          │   fake.server:4242 connection
         ║                              ║                                              ────────────────────────────
         ▼               │              ║                                          │
┌────────────────┐               ┌────────────┐                                            ┌──────────┐    ┌──────────┐
│     Actual     │       │       │    Ping    │      ┌───────────┐                 │       │ User 2's │    │ User 1's │
│fake.server:4242│──────────────▶│(keepalive)?│─no──▶│    Log    │────────────────────────▶┤IRC client├───▶│IRC client│
│     server     │       │       └────────────┘      └───────────┘                 │       └──────────┘    └──────────┘
└────────────────┘                                                            MPSC               ║               ║
         ▲               │                           ┌────────────────┐      queue │             ║               ║
         ╚═══════════════════════════════════════════│fake.server:4242│◀═════════════════════════╩═══════════════╝
                         │                           └────────────────┘            │
                          ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
```

## Connections
One connection will be maintained per `server:hostport` (the "server-facing" connection). Each user's `server:hostport` instance will be an additional connection (the "user-facing" connection). Messages will only be sent over user-facing connections after messages are persisted to the log.

New `server:hostports` will create new connections.

Each connection will be a thread.
